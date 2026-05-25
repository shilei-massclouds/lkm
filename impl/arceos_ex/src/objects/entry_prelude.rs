use core::arch::global_asm;

use crate::{arch::riscv64::csr, trace::Checkpoint};

use super::{
    boot_args::BootArgs,
    config::Config,
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
        self.kernel_image.preset(&self.lds)
    }

    pub fn root_stream_preset(&mut self) -> EventResult {
        self.root_stream.preset()
    }

    pub fn kernel_image_setup(&mut self) -> EventResult {
        self.kernel_image.setup(&self.lds)
    }

    pub fn cpu_group_preset(&mut self, boot_args: &BootArgs) -> EventResult {
        self.cpu_group.preset(boot_args)
    }

    pub fn init_task_preset(&mut self) -> EventResult {
        self.init_task.preset()
    }

    pub fn init_stack_preset(&mut self) -> EventResult {
        self.init_stack.preset(&self.lds)
    }

    pub fn event_stream_preset(&mut self) -> EventResult {
        self.event_stream.preset()
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

    fn global_pointer(&self) -> usize {
        global_pointer as usize
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

    fn init_stack_start(&self) -> usize {
        init_stack_start as usize
    }

    fn init_stack_end(&self) -> usize {
        init_stack_end as usize
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

    unsafe fn zero_bss(&self) {
        let start = self.bss_start() as *mut u8;
        let len = self.bss_end() - self.bss_start();
        unsafe {
            core::ptr::write_bytes(start, 0, len);
        }
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

    fn preset(&mut self, lds: &Lds) -> EventResult {
        if lds.state() != State::Online
            || !lds.entry_layout_ready()
            || csr::read_gp() != lds.global_pointer()
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

    fn setup(&mut self, lds: &Lds) -> EventResult {
        if self.lifecycle.state() == State::Prepared {
            unsafe {
                lds.zero_bss();
            }
        }

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::KernelImageReady,
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

    fn preset(&mut self) -> EventResult {
        csr::write_tp(core::ptr::addr_of!(INIT_TASK_STORAGE) as usize);
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::InitTaskPrepared,
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

    fn preset(&mut self, lds: &Lds) -> EventResult {
        if lds.init_stack_end() - lds.init_stack_start() < 4096
            || PT_SIZE_ON_STACK >= lds.init_stack_end() - lds.init_stack_start()
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

    fn preset(&mut self) -> EventResult {
        csr::write_stvec(early_event_entry as usize);
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::EventStreamPrepared,
        )
    }
}

#[repr(C)]
struct InitTaskStorage {
    reserved: usize,
}

static INIT_TASK_STORAGE: InitTaskStorage = InitTaskStorage { reserved: 0 };
