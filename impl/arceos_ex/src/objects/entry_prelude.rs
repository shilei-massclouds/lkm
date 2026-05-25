use core::arch::global_asm;

use crate::{arch::riscv64::csr, trace::Checkpoint};

use super::{
    boot_args::BootArgs,
    config::Config,
    entry_successor::MemBlock,
    fix_map::FixMap,
    raw_dtb::RawDtb,
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
}

#[unsafe(no_mangle)]
extern "C" fn early_event_entry_rust() -> ! {
    crate::arch::riscv64::sbi::putstr("arceos_ex early trap\n");
    crate::arch::riscv64::sbi::system_shutdown()
}

pub struct EntryPreludeObjects {
    config: Config,
    static_objects: StaticObjects,
    pub lds: Lds,
    interrupt_stream: InterruptStream,
    kernel_image: KernelImage,
    root_stream: RootStream,
    cpu_group: CpuGroup,
    init_task: InitTask,
    init_stack: InitStack,
    event_stream: EventStream,
    vm: Vm,
    raw_dtb: RawDtb,
    fix_map: FixMap,
}

impl EntryPreludeObjects {
    pub const fn new() -> Self {
        Self {
            config: Config::new(),
            static_objects: StaticObjects::new(),
            lds: Lds::new(),
            interrupt_stream: InterruptStream::new(),
            kernel_image: KernelImage::new(),
            root_stream: RootStream::new(),
            cpu_group: CpuGroup::new(),
            init_task: InitTask::new(),
            init_stack: InitStack::new(),
            event_stream: EventStream::new(),
            vm: Vm::new(),
            raw_dtb: RawDtb::new(),
            fix_map: FixMap::new(),
        }
    }

    pub fn interrupt_stream_preset(&mut self) -> EventResult {
        self.interrupt_stream.preset()
    }

    pub fn kernel_image_preset(&mut self) -> EventResult {
        self.kernel_image.preset(&self.config, &self.lds)
    }

    pub fn root_stream_preset(&mut self) -> EventResult {
        self.root_stream.preset()
    }

    pub fn kernel_image_setup(&mut self) -> EventResult {
        self.kernel_image.setup(&self.config, &self.lds)
    }

    pub fn cpu_group_preset(&mut self, boot_args: &BootArgs) -> EventResult {
        self.cpu_group.preset(boot_args)
    }

    pub fn init_task_preset(&mut self) -> EventResult {
        self.init_task.preset(&self.config)
    }

    pub fn init_stack_preset(&mut self) -> EventResult {
        self.init_stack.preset(&self.config, &self.lds)
    }

    pub fn event_stream_preset(&mut self) -> EventResult {
        self.event_stream.preset(&self.config)
    }

    pub fn vm_preset(&mut self, boot_args: &BootArgs) -> EventResult {
        self.vm.preset(
            &self.config,
            &mut self.static_objects,
            &self.lds,
            &self.kernel_image,
            boot_args,
            &mut self.raw_dtb,
            &mut self.fix_map,
        )
    }

    pub fn vm_setup(&mut self) -> ! {
        self.vm.setup(
            &self.config,
            &self.static_objects,
            &self.lds,
            &mut self.kernel_image,
        )
    }

    pub fn after_vm_setup(&mut self) -> EventResult {
        let result = self.event_stream.enable(&self.vm, &self.static_objects);
        if !result.is_success() {
            return result;
        }

        let result = self.init_task.enable(&self.config, &self.vm);
        if !result.is_success() {
            return result;
        }

        let result = self.init_stack.setup(&self.vm);
        if !result.is_success() {
            return result;
        }

        Soc::preset()
    }

    pub fn cleanup_entry_prelude_phase(&mut self) -> EventResult {
        crate::trace::checkpoint(Checkpoint::EntryPreludePhaseDestroyed);
        EventResult::Success
    }

    pub fn init_stack_enable(&mut self) -> EventResult {
        self.init_stack.enable()
    }

    pub fn interrupt_stream_setup(&mut self) -> EventResult {
        self.interrupt_stream.setup()
    }

    pub fn boot_cpu_setup(&mut self, boot_hartid_valid: bool) -> EventResult {
        self.cpu_group.boot_cpu_setup(boot_hartid_valid)
    }

    pub fn boot_cpu_enable(&mut self) -> EventResult {
        self.cpu_group.boot_cpu_enable()
    }

    pub fn boot_hartid(&self) -> usize {
        self.cpu_group.boot_hartid()
    }

    pub fn raw_dtb(&self) -> &RawDtb {
        &self.raw_dtb
    }

    pub fn fix_map(&self) -> &FixMap {
        &self.fix_map
    }

    pub fn kernel_image(&self) -> &KernelImage {
        &self.kernel_image
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn vm_enable(&mut self, memblock: &MemBlock) -> EventResult {
        self.vm.enable(
            &self.config,
            &mut self.static_objects,
            &self.lds,
            &self.kernel_image,
            memblock,
        )
    }

    pub fn vm_state(&self) -> State {
        self.vm.state()
    }
}

pub struct Lds {
    lifecycle: Lifecycle,
}

impl Lds {
    const fn new() -> Self {
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

    fn entry_layout_ready(&self) -> bool {
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

    unsafe fn zero_bss(&self, config: &Config) -> bool {
        let Some(start) = config.runtime_to_phys(self.bss_start()) else {
            return false;
        };
        let len = self.bss_end() - self.bss_start();
        let start = start as *mut u8;
        unsafe {
            core::ptr::write_bytes(start, 0, len);
        }
        true
    }
}

pub struct InterruptStream {
    lifecycle: Lifecycle,
}

impl InterruptStream {
    const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
        }
    }

    fn preset(&mut self) -> EventResult {
        csr::close_interrupt_stream();
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::InterruptStreamPrepared,
        )
    }

    fn setup(&mut self) -> EventResult {
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
    const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    fn preset(&mut self, config: &Config, lds: &Lds) -> EventResult {
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

        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::KernelImagePrepared,
        )
    }

    fn setup(&mut self, config: &Config, lds: &Lds) -> EventResult {
        if self.lifecycle.state() == State::Prepared {
            unsafe {
                if !lds.zero_bss(config) {
                    return EventResult::failed_condition(
                        LifecycleEvent::Setup,
                        self.lifecycle.state(),
                        State::Prepared,
                        State::Ready,
                    );
                }
            }
        }

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::KernelImageReady,
        )
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
    const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
        }
    }

    fn preset(&mut self) -> EventResult {
        csr::disable_kernel_fpu_vector();
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::RootStreamPrepared,
        )
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

    fn preset(&mut self, boot_args: &BootArgs) -> EventResult {
        self.hartid = boot_args.boot_hartid();
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::BootCpuPrepared,
        )
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
}

pub struct CpuGroup {
    lifecycle: Lifecycle,
    boot_cpu: BootCpu,
}

impl CpuGroup {
    const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            boot_cpu: BootCpu::new(),
        }
    }

    fn preset(&mut self, boot_args: &BootArgs) -> EventResult {
        let result = self.boot_cpu.preset(boot_args);
        if !result.is_success() {
            return result;
        }

        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::CpuGroupPrepared,
        )
    }

    fn boot_cpu_setup(&mut self, boot_hartid_valid: bool) -> EventResult {
        self.boot_cpu.setup(boot_hartid_valid)
    }

    fn boot_cpu_enable(&mut self) -> EventResult {
        self.boot_cpu.enable()
    }

    fn boot_hartid(&self) -> usize {
        self.boot_cpu.hartid
    }
}

pub struct InitTask {
    lifecycle: Lifecycle,
}

impl InitTask {
    const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
        }
    }

    fn preset(&mut self, config: &Config) -> EventResult {
        let Some(init_task_phys) =
            config.runtime_to_phys(core::ptr::addr_of!(INIT_TASK_STORAGE) as usize)
        else {
            return EventResult::failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        };
        csr::write_tp(init_task_phys);
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::InitTaskPrepared,
        )
    }

    fn enable(&mut self, config: &Config, vm: &Vm) -> EventResult {
        let Some(init_task_virt) =
            config.runtime_to_link(core::ptr::addr_of!(INIT_TASK_STORAGE) as usize)
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
}

pub struct InitStack {
    lifecycle: Lifecycle,
}

impl InitStack {
    const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
        }
    }

    fn preset(&mut self, config: &Config, lds: &Lds) -> EventResult {
        if lds.init_stack_end() - lds.init_stack_start() < 4096
            || PT_SIZE_ON_STACK >= lds.init_stack_end() - lds.init_stack_start()
            || lds.init_stack_end_phys(config).is_none()
        {
            return EventResult::failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::InitStackPrepared,
        )
    }

    fn setup(&mut self, vm: &Vm) -> EventResult {
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

    fn enable(&mut self) -> EventResult {
        self.lifecycle.transition(
            LifecycleEvent::Enable,
            State::Ready,
            State::Online,
            Checkpoint::InitStackOnline,
        )
    }
}

pub struct EventStream {
    lifecycle: Lifecycle,
}

impl EventStream {
    const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
        }
    }

    fn preset(&mut self, config: &Config) -> EventResult {
        let Some(early_event_entry_phys) = config.runtime_to_phys(early_event_entry as usize) else {
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

    fn enable(&mut self, vm: &Vm, static_objects: &StaticObjects) -> EventResult {
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
    fn preset() -> EventResult {
        crate::trace::checkpoint(Checkpoint::SocPrepared);
        EventResult::Success
    }
}

#[repr(C)]
struct InitTaskStorage {
    reserved: usize,
}

static INIT_TASK_STORAGE: InitTaskStorage = InitTaskStorage { reserved: 0 };
