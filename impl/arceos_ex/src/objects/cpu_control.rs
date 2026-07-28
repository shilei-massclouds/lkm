use super::{
    boot_task::BootTask,
    interrupt_type::InterruptType,
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
};
use crate::checkpoint::Checkpoint;

pub struct PreemptionControl {
    lifecycle: Lifecycle,
    disable_depth: usize,
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
impl PreemptionControl {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            disable_depth: 0,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn enabled(&self) -> bool {
        self.disable_depth == 0
    }

    pub const fn disabled(&self) -> bool {
        self.disable_depth != 0
    }

    pub fn setup(&mut self, boot_task: &BootTask) -> EventResult {
        self.setup_with_depth(
            boot_task,
            0,
            crate::checkpoint::Checkpoint::BootIdlePreemptionReady,
        )
    }

    pub fn setup_disabled(&mut self, boot_task: &BootTask) -> EventResult {
        self.setup_with_depth(
            boot_task,
            1,
            crate::checkpoint::Checkpoint::BootIdlePreemptionReady,
        )
    }

    pub fn setup_disabled_with_checkpoint(
        &mut self,
        boot_task: &BootTask,
        checkpoint: Checkpoint,
    ) -> EventResult {
        self.setup_with_depth(boot_task, 1, checkpoint)
    }

    fn setup_with_depth(
        &mut self,
        boot_task: &BootTask,
        disable_depth: usize,
        checkpoint: Checkpoint,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base || !boot_task.online() {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.disable_depth = disable_depth;
        self.lifecycle
            .transition(LifecycleEvent::Setup, State::Base, State::Ready, checkpoint)
    }

    pub fn disable(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Ready {
            return failed_condition(
                LifecycleEvent::Disable,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        self.disable_depth = self.disable_depth.wrapping_add(1);
        Ok(())
    }

    pub fn enable(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Ready || self.disable_depth == 0 {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        self.disable_depth -= 1;
        Ok(())
    }

    pub fn enable_no_resched(&mut self) -> EventResult {
        self.enable()
    }
}

pub struct RawSpinLock {
    lifecycle: Lifecycle,
    locked: bool,
    acquired_count: usize,
    released_count: usize,
    irqsave_entered_count: usize,
    irqrestore_exited_count: usize,
    irqrestore_restored_before_preemption_enabled: bool,
}

impl RawSpinLock {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            locked: false,
            acquired_count: 0,
            released_count: 0,
            irqsave_entered_count: 0,
            irqrestore_exited_count: 0,
            irqrestore_restored_before_preemption_enabled: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn locked(&self) -> bool {
        self.locked
    }

    pub const fn acquired_count(&self) -> usize {
        self.acquired_count
    }

    pub const fn released_count(&self) -> usize {
        self.released_count
    }

    pub const fn irqsave_entered_count(&self) -> usize {
        self.irqsave_entered_count
    }

    pub const fn irqrestore_exited_count(&self) -> usize {
        self.irqrestore_exited_count
    }

    pub const fn irqrestore_restored_before_preemption_enabled(&self) -> bool {
        self.irqrestore_restored_before_preemption_enabled
    }

    pub fn setup(&mut self) -> EventResult {
        self.setup_with_checkpoint(Checkpoint::KernelInitTaskPiLockReady)
    }

    pub fn setup_with_checkpoint(&mut self, checkpoint: Checkpoint) -> EventResult {
        if self.lifecycle.state() != State::Base {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.locked = false;
        self.lifecycle
            .transition(LifecycleEvent::Setup, State::Base, State::Ready, checkpoint)
    }

    pub fn acquire(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Ready || self.locked {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        self.locked = true;
        self.acquired_count = self.acquired_count.wrapping_add(1);
        Ok(())
    }

    pub fn release(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Ready || !self.locked {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        self.locked = false;
        self.released_count = self.released_count.wrapping_add(1);
        Ok(())
    }

    pub fn lock_irqsave(
        &mut self,
        local_interrupt: &mut InterruptType,
        preemption: &mut PreemptionControl,
    ) -> EventResult {
        if self.lifecycle.state() != State::Ready || self.locked {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        local_interrupt.save_and_disable()?;
        preemption.disable()?;
        self.acquire()?;
        self.irqsave_entered_count = self.irqsave_entered_count.wrapping_add(1);
        Ok(())
    }

    pub fn unlock_irqrestore(
        &mut self,
        local_interrupt: &mut InterruptType,
        preemption: &mut PreemptionControl,
    ) -> EventResult {
        if self.lifecycle.state() != State::Ready || !self.locked {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        self.release()?;
        let restored_before = local_interrupt.restored_count();
        local_interrupt.restore()?;
        self.irqrestore_restored_before_preemption_enabled = local_interrupt.restored_count()
            == restored_before.wrapping_add(1)
            && preemption.disabled();
        preemption.enable()?;
        self.irqrestore_exited_count = self.irqrestore_exited_count.wrapping_add(1);
        Ok(())
    }
}

pub struct RcuReadSide {
    lifecycle: Lifecycle,
    held_depth: usize,
    read_lock_count: usize,
    read_unlock_count: usize,
    incomplete_first_slice: bool,
    full_semantics_deferred: bool,
}

impl RcuReadSide {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            held_depth: 0,
            read_lock_count: 0,
            read_unlock_count: 0,
            incomplete_first_slice: false,
            full_semantics_deferred: true,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn read_lock_count(&self) -> usize {
        self.read_lock_count
    }

    pub const fn read_unlock_count(&self) -> usize {
        self.read_unlock_count
    }

    pub const fn incomplete_first_slice(&self) -> bool {
        self.incomplete_first_slice
    }

    pub const fn full_semantics_deferred(&self) -> bool {
        self.full_semantics_deferred
    }

    pub const fn balanced(&self) -> bool {
        self.held_depth == 0 && self.read_lock_count == self.read_unlock_count
    }

    pub fn preset_incomplete_first_slice(&mut self, checkpoint: Checkpoint) -> EventResult {
        if self.lifecycle.state() != State::Base {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.held_depth = 0;
        self.read_lock_count = 0;
        self.read_unlock_count = 0;
        self.incomplete_first_slice = true;
        self.full_semantics_deferred = true;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            checkpoint,
        )
    }

    pub fn read_lock(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Prepared || !self.incomplete_first_slice {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Prepared,
                State::Prepared,
            );
        }

        self.held_depth = self.held_depth.wrapping_add(1);
        self.read_lock_count = self.read_lock_count.wrapping_add(1);
        Ok(())
    }

    pub fn read_unlock(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Prepared || self.held_depth == 0 {
            return failed_condition(
                LifecycleEvent::Disable,
                self.lifecycle.state(),
                State::Prepared,
                State::Prepared,
            );
        }

        self.held_depth -= 1;
        self.read_unlock_count = self.read_unlock_count.wrapping_add(1);
        Ok(())
    }
}
