use super::{
    boot_args::BootArgs,
    cpu::{Cpu, CpuView},
    cpu_group::CpuGroup,
    init_task::InitTask,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
};
use crate::arch::riscv64::csr;
use crate::trace::Checkpoint;

const LOCAL_INTERRUPT_SAVE_STACK: usize = 8;

#[allow(dead_code)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum CurrentTaskRef {
    None,
    BootIdle,
    KernelInit,
    Kthreadd,
    SmokeScheduler,
    SmokeMutex,
    SmokeRwsem,
    SmokeRwLock,
}

#[allow(dead_code)]
impl CurrentTaskRef {
    pub const fn name(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::BootIdle => "BootIdleTask",
            Self::KernelInit => "KernelInitTask",
            Self::Kthreadd => "KthreaddTask",
            Self::SmokeScheduler => "SmokeSchedulerTask",
            Self::SmokeMutex => "SmokeMutexTask",
            Self::SmokeRwsem => "SmokeRwsemTask",
            Self::SmokeRwLock => "SmokeRwLockTask",
        }
    }
}

pub struct BootCurrentCpu {
    lifecycle: Lifecycle,
    boot_cpu: Cpu,
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
            boot_cpu: Cpu::boot(),
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

    pub fn boot_cpu(&self) -> Option<CpuView> {
        self.boot_cpu.view()
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

        self.boot_cpu.adopt_head_preset(boot_args)?;
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

    pub fn setup_boot_cpu(&mut self, boot_hartid_valid: bool) -> EventResult {
        self.boot_cpu.setup_boot(boot_hartid_valid)
    }

    pub fn enable_boot_cpu(&mut self) -> EventResult {
        self.boot_cpu.enable_boot()
    }

    pub fn enable(&mut self, cpu_group: &CpuGroup) -> EventResult {
        let boot_cpu = cpu_group.boot_cpu();
        if self.lifecycle.state() != State::Ready
            || cpu_group.state() != State::Prepared
            || boot_cpu.map(|cpu| cpu.hartid()) != Some(self.hartid)
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
    switch_committed_count: usize,
}

impl CurrentTaskSlot {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            current: CurrentTaskRef::None,
            switch_committed_count: 0,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn current(&self) -> CurrentTaskRef {
        self.current
    }

    pub const fn switch_committed_count(&self) -> usize {
        self.switch_committed_count
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
        self.set_current(CurrentTaskRef::BootIdle)
    }

    pub fn set_current(&mut self, task_ref: CurrentTaskRef) -> EventResult {
        if self.lifecycle.state() != State::Ready {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        if matches!(task_ref, CurrentTaskRef::None) {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        self.current = task_ref;
        Ok(())
    }

    pub fn commit_switch_to(&mut self, next_ref: CurrentTaskRef) -> EventResult {
        if self.lifecycle.state() != State::Ready || matches!(next_ref, CurrentTaskRef::None) {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        self.current = next_ref;
        self.switch_committed_count = self.switch_committed_count.wrapping_add(1);
        Ok(())
    }

    pub const fn current_is_boot_idle(&self) -> bool {
        matches!(self.current, CurrentTaskRef::BootIdle)
    }

    pub const fn current_is_kernel_init(&self) -> bool {
        matches!(self.current, CurrentTaskRef::KernelInit)
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
        self.setup_with_depth(
            init_task,
            0,
            crate::trace::Checkpoint::BootIdlePreemptionReady,
        )
    }

    pub fn setup_disabled(&mut self, init_task: &InitTask) -> EventResult {
        self.setup_with_depth(
            init_task,
            1,
            crate::trace::Checkpoint::BootIdlePreemptionReady,
        )
    }

    pub fn setup_disabled_with_checkpoint(
        &mut self,
        init_task: &InitTask,
        checkpoint: Checkpoint,
    ) -> EventResult {
        self.setup_with_depth(init_task, 1, checkpoint)
    }

    fn setup_with_depth(
        &mut self,
        init_task: &InitTask,
        disable_depth: usize,
        checkpoint: Checkpoint,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base || init_task.state() != State::Online {
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
        self.acquire()?;
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
