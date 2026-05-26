use core::arch::global_asm;

use crate::{arch::riscv64::csr, trace::Checkpoint};

use super::{
    boot_args::BootArgs,
    config::Config,
    state::{EventResult, Lifecycle, LifecycleEvent, State},
    static_objects::StaticObjects,
    vm::Vm,
};

pub const PT_SIZE_ON_STACK: usize = 256;

global_asm!(
    r#"
    .section .text.trap, "ax"
    .align 2
    .globl early_event_entry
early_event_entry:
    j early_event_entry_rust

    .section .bss.objects, "aw", @nobits
    .align 3
    .globl bss_anchor
bss_anchor:
    .space 8
"#
);

unsafe extern "C" {
    fn _start();
    fn kernel_start();
    fn kernel_end();
    #[link_name = "__global_pointer$"]
    fn global_pointer();
    fn __head_text_start();
    fn __head_text_end();
    fn _sbss();
    fn _ebss();
    fn init_stack_start();
    fn init_stack_end();
    fn early_event_entry();
    static head_boot_hartid: usize;
    static head_init_stack_sp: usize;
}

#[unsafe(no_mangle)]
extern "C" fn early_event_entry_rust() -> ! {
    crate::arch::riscv64::sbi::putstr("arceos_ex early trap\n");
    crate::arch::riscv64::sbi::system_shutdown()
}

pub struct Lds {
    lifecycle: Lifecycle,
}

impl Lds {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Online),
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub fn kernel_start(&self) -> usize {
        kernel_start as usize
    }

    pub fn kernel_end(&self) -> usize {
        kernel_end as usize
    }

    fn text_start(&self) -> usize {
        _start as usize
    }

    pub fn global_pointer(&self) -> usize {
        global_pointer as usize
    }

    pub fn current_global_pointer(&self, config: &Config) -> Option<usize> {
        if csr::read_satp() == 0 {
            config.runtime_to_phys(self.global_pointer())
        } else {
            Some(self.global_pointer())
        }
    }

    fn head_text_start(&self) -> usize {
        __head_text_start as usize
    }

    fn head_text_end(&self) -> usize {
        __head_text_end as usize
    }

    fn bss_start(&self) -> usize {
        _sbss as usize
    }

    fn bss_end(&self) -> usize {
        _ebss as usize
    }

    pub fn init_stack_start(&self) -> usize {
        init_stack_start as usize
    }

    pub fn init_stack_end(&self) -> usize {
        init_stack_end as usize
    }

    pub fn init_stack_end_phys(&self, config: &Config) -> Option<usize> {
        config.runtime_to_phys(self.init_stack_end())
    }

    pub fn entry_layout_ready(&self) -> bool {
        let kernel_start_addr = self.kernel_start();
        let kernel_end_addr = self.kernel_end();
        let head_start = self.head_text_start();
        let head_end = self.head_text_end();

        self.text_start() == kernel_start_addr
            && head_start == kernel_start_addr
            && head_end > head_start
            && head_end <= kernel_end_addr
            && self.global_pointer() != 0
            && self.bss_end() > self.bss_start()
            && self.bss_start() >= kernel_start_addr
            && self.bss_end() <= kernel_end_addr
            && self.init_stack_end() > self.init_stack_start()
            && self.init_stack_start() >= kernel_start_addr
            && self.init_stack_end() <= kernel_end_addr
            && self.init_stack_start() & 0xfff == 0
            && self.init_stack_end() & 0xfff == 0
    }

    fn bss_zeroed(&self, config: &Config) -> bool {
        let Some(start) = config.runtime_to_phys(self.bss_start()) else {
            return false;
        };
        let len = self.bss_end() - self.bss_start();
        let bytes = unsafe { core::slice::from_raw_parts(start as *const u8, len) };
        bytes.iter().all(|byte| *byte == 0)
    }
}

pub struct InterruptStream {
    lifecycle: Lifecycle,
}

impl InterruptStream {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
        }
    }

    pub fn adopt_head_preset(&mut self) -> EventResult {
        if csr::read_sie() != 0 || csr::read_sip() != 0 {
            return EventResult::failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.lifecycle
            .adopt_transition(LifecycleEvent::Preset, State::Base, State::Prepared)
    }

    pub fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub fn setup(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Prepared {
            return EventResult::failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::InterruptStreamReady,
        )
    }
}

pub struct KernelImage {
    lifecycle: Lifecycle,
}

impl KernelImage {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub fn adopt_head_preset(&mut self, config: &Config, lds: &Lds) -> EventResult {
        if lds.state() != State::Online
            || !lds.entry_layout_ready()
            || lds.current_global_pointer(config) != Some(csr::read_gp())
        {
            return EventResult::failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.lifecycle
            .adopt_transition(LifecycleEvent::Preset, State::Base, State::Prepared)
    }

    pub fn adopt_head_setup(&mut self, config: &Config, lds: &Lds) -> EventResult {
        if self.lifecycle.state() != State::Prepared || !lds.bss_zeroed(config) {
            return EventResult::failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }

        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Prepared, State::Ready)
    }

    pub fn enable(&mut self, lds: &Lds) -> EventResult {
        if self.lifecycle.state() != State::Ready || csr::read_gp() != lds.global_pointer() {
            return EventResult::failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }

        self.lifecycle.transition(
            LifecycleEvent::Enable,
            State::Ready,
            State::Online,
            Checkpoint::KernelImageOnline,
        )
    }
}

pub struct RootStream {
    lifecycle: Lifecycle,
}

impl RootStream {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
        }
    }

    pub fn adopt_head_preset(&mut self) -> EventResult {
        if !csr::kernel_fpu_vector_disabled() {
            return EventResult::failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.lifecycle
            .adopt_transition(LifecycleEvent::Preset, State::Base, State::Prepared)
    }

    pub fn state(&self) -> State {
        self.lifecycle.state()
    }
}

pub struct BootCpu {
    lifecycle: Lifecycle,
    hartid: usize,
}

impl BootCpu {
    const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            hartid: usize::MAX,
        }
    }

    fn adopt_head_preset(&mut self, boot_args: &BootArgs) -> EventResult {
        let head_hartid = unsafe { core::ptr::addr_of!(head_boot_hartid).read_volatile() };
        if head_hartid != boot_args.boot_hartid() {
            return EventResult::failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.hartid = head_hartid;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Preset, State::Base, State::Prepared)
    }

    fn setup(&mut self, boot_hartid_valid: bool) -> EventResult {
        if self.lifecycle.state() != State::Prepared || !boot_hartid_valid {
            return EventResult::failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::BootCpuReady,
        )
    }

    fn enable(&mut self) -> EventResult {
        self.lifecycle.transition(
            LifecycleEvent::Enable,
            State::Ready,
            State::Online,
            Checkpoint::BootCpuOnline,
        )
    }

    fn state(&self) -> State {
        self.lifecycle.state()
    }
}

pub struct CpuGroup {
    lifecycle: Lifecycle,
    boot_cpu: BootCpu,
}

impl CpuGroup {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            boot_cpu: BootCpu::new(),
        }
    }

    pub fn adopt_head_preset(&mut self, boot_args: &BootArgs) -> EventResult {
        let result = self.boot_cpu.adopt_head_preset(boot_args);
        if !result.is_success() {
            return result;
        }

        self.lifecycle
            .adopt_transition(LifecycleEvent::Preset, State::Base, State::Prepared)
    }

    pub fn boot_cpu_setup(&mut self, boot_hartid_valid: bool) -> EventResult {
        self.boot_cpu.setup(boot_hartid_valid)
    }

    pub fn boot_cpu_enable(&mut self) -> EventResult {
        self.boot_cpu.enable()
    }

    pub fn boot_hartid(&self) -> usize {
        self.boot_cpu.hartid
    }

    pub fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub fn boot_cpu_state(&self) -> State {
        self.boot_cpu.state()
    }
}

pub struct InitTask {
    lifecycle: Lifecycle,
}

impl InitTask {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
        }
    }

    pub fn adopt_head_preset(&mut self, config: &Config) -> EventResult {
        let Some(init_task_phys) =
            config.runtime_to_phys(core::ptr::addr_of!(init_task_storage) as usize)
        else {
            return EventResult::failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        };

        if csr::read_tp() != init_task_phys {
            return EventResult::failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.lifecycle
            .adopt_transition(LifecycleEvent::Preset, State::Base, State::Prepared)
    }

    pub fn enable(&mut self, config: &Config, vm: &Vm) -> EventResult {
        let Some(init_task_virt) =
            config.runtime_to_link(core::ptr::addr_of!(init_task_storage) as usize)
        else {
            return EventResult::failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Prepared,
                State::Online,
            );
        };

        if self.lifecycle.state() != State::Prepared || vm.state() != State::Ready {
            return EventResult::failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Prepared,
                State::Online,
            );
        }

        csr::write_tp(init_task_virt);
        self.lifecycle.transition(
            LifecycleEvent::Enable,
            State::Prepared,
            State::Online,
            Checkpoint::InitTaskOnline,
        )
    }

    pub fn state(&self) -> State {
        self.lifecycle.state()
    }
}

pub struct InitStack {
    lifecycle: Lifecycle,
}

impl InitStack {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
        }
    }

    pub fn adopt_head_preset(&mut self, config: &Config, lds: &Lds) -> EventResult {
        let head_sp = unsafe { core::ptr::addr_of!(head_init_stack_sp).read_volatile() };
        let Some(stack_phys) = lds.init_stack_end_phys(config).and_then(|stack_end| {
            stack_end
                .checked_sub(PT_SIZE_ON_STACK)
                .filter(|stack| *stack == head_sp)
        }) else {
            return EventResult::failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        };

        if stack_phys < lds.init_stack_start() || stack_phys >= lds.init_stack_end() {
            return EventResult::failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.lifecycle
            .adopt_transition(LifecycleEvent::Preset, State::Base, State::Prepared)
    }

    pub fn setup(&mut self, vm: &Vm) -> EventResult {
        if self.lifecycle.state() != State::Prepared || vm.state() != State::Ready {
            return EventResult::failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::InitStackReady,
        )
    }

    pub fn enable(&mut self) -> EventResult {
        self.lifecycle.transition(
            LifecycleEvent::Enable,
            State::Ready,
            State::Online,
            Checkpoint::InitStackOnline,
        )
    }

    pub fn state(&self) -> State {
        self.lifecycle.state()
    }
}

pub struct EventStream {
    lifecycle: Lifecycle,
}

impl EventStream {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
        }
    }

    pub fn preset(&mut self, config: &Config) -> EventResult {
        let Some(early_event_entry_phys) = config.runtime_to_phys(early_event_entry as usize)
        else {
            return EventResult::failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        };
        csr::write_stvec(early_event_entry_phys);
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::EventStreamPrepared,
        )
    }

    pub fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub fn enable(&mut self, vm: &Vm, static_objects: &StaticObjects) -> EventResult {
        let _ = static_objects.state();
        if self.lifecycle.state() != State::Prepared || vm.state() != State::Ready {
            return EventResult::failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Prepared,
                State::Online,
            );
        }

        csr::write_stvec(early_event_entry as usize);
        csr::clear_sscratch();
        self.lifecycle.transition(
            LifecycleEvent::Enable,
            State::Prepared,
            State::Online,
            Checkpoint::EventStreamOnline,
        )
    }
}

pub struct Soc;

impl Soc {
    pub fn preset() -> EventResult {
        crate::trace::checkpoint(Checkpoint::SocPrepared);
        EventResult::Success
    }
}

#[repr(C)]
pub struct InitTaskStorage {
    reserved: usize,
}

#[unsafe(no_mangle)]
pub static init_task_storage: InitTaskStorage = InitTaskStorage { reserved: 0 };
