use super::{
    boot_args::BootArgs,
    cpu_group::CpuGroup,
    init_task::InitTask,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
};
use crate::arch::riscv64::csr;

const LOCAL_INTERRUPT_SAVE_STACK: usize = 8;

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum CurrentTaskRef {
    None,
    BootIdle,
}

pub struct BootCurrentCpu {
    lifecycle: Lifecycle,
    hartid: usize,
    logical_id: usize,
    owns_boot_cpu: bool,
    bootstrap_role_ready: bool,
    registered_in_cpu_group: bool,
}

impl BootCurrentCpu {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            hartid: usize::MAX,
            logical_id: usize::MAX,
            owns_boot_cpu: false,
            bootstrap_role_ready: false,
            registered_in_cpu_group: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn hartid(&self) -> usize {
        self.hartid
    }

    pub const fn logical_id(&self) -> usize {
        self.logical_id
    }

    pub const fn owns_boot_cpu(&self) -> bool {
        self.owns_boot_cpu
    }

    pub const fn bootstrap_role_ready(&self) -> bool {
        self.bootstrap_role_ready
    }

    pub const fn registered_in_cpu_group(&self) -> bool {
        self.registered_in_cpu_group
    }

    pub fn adopt_head_preset(&mut self, boot_args: &BootArgs) -> EventResult {
        if self.lifecycle.state() != State::Base {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.hartid = boot_args.boot_hartid();
        self.owns_boot_cpu = true;
        self.bootstrap_role_ready = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Preset, State::Base, State::Prepared)
    }

    pub fn setup(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Prepared || !self.owns_boot_cpu {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }

        self.logical_id = 0;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            crate::trace::Checkpoint::BootCurrentCpuReady,
        )
    }

    pub fn enable(&mut self, cpu_group: &CpuGroup) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || cpu_group.state() != State::Prepared
            || cpu_group.boot_hartid() != self.hartid
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }

        self.registered_in_cpu_group = true;
        self.lifecycle.transition(
            LifecycleEvent::Enable,
            State::Ready,
            State::Online,
            crate::trace::Checkpoint::BootCurrentCpuOnline,
        )
    }
}

pub struct LocalInterruptControl {
    lifecycle: Lifecycle,
    enabled: bool,
    saved_enabled_stack: [bool; LOCAL_INTERRUPT_SAVE_STACK],
    save_depth: usize,
    saved_and_disabled_count: usize,
    restored_count: usize,
}

impl LocalInterruptControl {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            enabled: false,
            saved_enabled_stack: [false; LOCAL_INTERRUPT_SAVE_STACK],
            save_depth: 0,
            saved_and_disabled_count: 0,
            restored_count: 0,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn enabled(&self) -> bool {
        self.enabled
    }

    pub const fn disabled(&self) -> bool {
        !self.enabled
    }

    pub const fn saved_and_disabled_count(&self) -> usize {
        self.saved_and_disabled_count
    }

    pub const fn restored_count(&self) -> usize {
        self.restored_count
    }

    pub fn setup(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Base {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        csr::disable_supervisor_interrupts();
        self.enabled = false;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            crate::trace::Checkpoint::BootCpuLocalInterruptReady,
        )
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

        csr::disable_supervisor_interrupts();
        self.enabled = false;
        Ok(())
    }

    pub fn enable(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Ready {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        csr::enable_supervisor_interrupts();
        self.enabled = csr::supervisor_interrupts_enabled();
        if !self.enabled {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }
        Ok(())
    }

    pub fn save_and_disable(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Ready {
            return failed_condition(
                LifecycleEvent::Disable,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        if self.save_depth == self.saved_enabled_stack.len() {
            return failed_condition(
                LifecycleEvent::Disable,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        self.saved_enabled_stack[self.save_depth] = csr::supervisor_interrupts_enabled();
        self.save_depth += 1;
        csr::disable_supervisor_interrupts();
        self.enabled = false;
        self.saved_and_disabled_count = self.saved_and_disabled_count.wrapping_add(1);
        Ok(())
    }

    pub fn restore(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Ready || self.save_depth == 0 {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        self.save_depth -= 1;
        if self.saved_enabled_stack[self.save_depth] {
            csr::enable_supervisor_interrupts();
        } else {
            csr::disable_supervisor_interrupts();
        }
        self.enabled = csr::supervisor_interrupts_enabled();
        self.restored_count = self.restored_count.wrapping_add(1);
        Ok(())
    }
}

pub struct CurrentTaskSlot {
    lifecycle: Lifecycle,
    current: CurrentTaskRef,
}

impl CurrentTaskSlot {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            current: CurrentTaskRef::None,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn current(&self) -> CurrentTaskRef {
        self.current
    }

    pub fn setup(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Base {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            crate::trace::Checkpoint::BootCpuCurrentTaskReady,
        )
    }

    pub fn set_current_boot_idle(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Ready {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        self.current = CurrentTaskRef::BootIdle;
        Ok(())
    }

    pub const fn current_is_boot_idle(&self) -> bool {
        matches!(self.current, CurrentTaskRef::BootIdle)
    }
}

pub struct PreemptionControl {
    lifecycle: Lifecycle,
    disable_depth: usize,
}

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

    pub fn setup(&mut self, init_task: &InitTask) -> EventResult {
        if self.lifecycle.state() != State::Base || init_task.state() != State::Online {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.disable_depth = 0;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            crate::trace::Checkpoint::BootIdlePreemptionReady,
        )
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
}

pub struct RawSpinLock {
    lifecycle: Lifecycle,
    locked: bool,
    irqsave_entered_count: usize,
    irqrestore_exited_count: usize,
    irqrestore_restored_before_preemption_enabled: bool,
}

impl RawSpinLock {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            locked: false,
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
        if self.lifecycle.state() != State::Base {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.locked = false;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            crate::trace::Checkpoint::KernelInitTaskPiLockReady,
        )
    }

    pub fn lock_irqsave(
        &mut self,
        local_interrupt: &mut LocalInterruptControl,
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
        self.locked = true;
        self.irqsave_entered_count = self.irqsave_entered_count.wrapping_add(1);
        Ok(())
    }

    pub fn unlock_irqrestore(
        &mut self,
        local_interrupt: &mut LocalInterruptControl,
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

        self.locked = false;
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
