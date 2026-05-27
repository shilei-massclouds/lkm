use crate::{arch::riscv64::csr, trace::Checkpoint};

use super::{
    boot_args::BootArgs,
    config::Config,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
    vm::Vm,
};

pub const PT_SIZE_ON_STACK: usize = 256;

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
    static head_boot_hartid: usize;
    static head_init_stack_sp: usize;
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

    pub fn current_global_pointer(&self, kernel_image: &KernelImage) -> Option<usize> {
        if csr::read_satp() == 0 {
            kernel_image.runtime_to_phys(self.global_pointer())
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

    pub fn init_stack_end_phys(&self, kernel_image: &KernelImage) -> Option<usize> {
        kernel_image.runtime_to_phys(self.init_stack_end())
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

    fn bss_zeroed(&self, kernel_image: &KernelImage) -> bool {
        let Some(start) = kernel_image.runtime_to_phys(self.bss_start()) else {
            return false;
        };
        let len = self.bss_end() - self.bss_start();
        let bytes = unsafe { core::slice::from_raw_parts(start as *const u8, len) };
        bytes.iter().all(|byte| *byte == 0)
    }
}

pub struct KernelImage {
    lifecycle: Lifecycle,
    phys_start: usize,
    virt_start: usize,
    virt_offset: usize,
}

impl KernelImage {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            phys_start: 0,
            virt_start: 0,
            virt_offset: 0,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn phys_start(&self) -> usize {
        self.phys_start
    }

    pub const fn virt_start(&self) -> usize {
        self.virt_start
    }

    pub const fn virt_offset(&self) -> usize {
        self.virt_offset
    }

    pub fn link_to_phys(&self, addr: usize) -> Option<usize> {
        addr.checked_sub(self.virt_offset)
    }

    pub fn phys_to_link(&self, addr: usize) -> Option<usize> {
        addr.checked_add(self.virt_offset)
    }

    pub fn runtime_to_phys(&self, addr: usize) -> Option<usize> {
        if addr >= self.virt_start {
            self.link_to_phys(addr)
        } else {
            Some(addr)
        }
    }

    pub fn runtime_to_link(&self, addr: usize) -> Option<usize> {
        if addr >= self.virt_start {
            Some(addr)
        } else {
            self.phys_to_link(addr)
        }
    }

    pub fn adopt_head_preset(&mut self, config: &Config, lds: &Lds) -> EventResult {
        let runtime_start = lds.kernel_start();
        let virt_start = config.kernel_link_addr();
        let Some(virt_offset) = virt_start.checked_sub(runtime_start) else {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        };

        self.phys_start = runtime_start;
        self.virt_start = virt_start;
        self.virt_offset = virt_offset;

        if lds.state() != State::Online
            || !lds.entry_layout_ready()
            || runtime_start == 0
            || runtime_start >= virt_start
            || !runtime_start.is_multiple_of(config.pmd_size())
            || lds.current_global_pointer(self) != Some(csr::read_gp())
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.lifecycle
            .adopt_transition(LifecycleEvent::Preset, State::Base, State::Prepared)
    }

    pub fn adopt_head_setup(&mut self, lds: &Lds) -> EventResult {
        if self.lifecycle.state() != State::Prepared || !lds.bss_zeroed(self) {
            return failed_condition(
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
            return failed_condition(
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
            return failed_condition(
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
            return failed_condition(
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
        if result.is_err() {
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

    pub fn adopt_head_preset(&mut self, kernel_image: &KernelImage) -> EventResult {
        let Some(init_task_phys) =
            kernel_image.runtime_to_phys(core::ptr::addr_of!(init_task_storage) as usize)
        else {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        };

        if csr::read_tp() != init_task_phys {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.lifecycle
            .adopt_transition(LifecycleEvent::Preset, State::Base, State::Prepared)
    }

    pub fn enable(&mut self, kernel_image: &KernelImage, vm: &Vm) -> EventResult {
        let Some(init_task_virt) =
            kernel_image.runtime_to_link(core::ptr::addr_of!(init_task_storage) as usize)
        else {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Prepared,
                State::Online,
            );
        };

        if self.lifecycle.state() != State::Prepared || vm.state() != State::Ready {
            return failed_condition(
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

    pub fn adopt_head_preset(&mut self, kernel_image: &KernelImage, lds: &Lds) -> EventResult {
        let head_sp = unsafe { core::ptr::addr_of!(head_init_stack_sp).read_volatile() };
        let Some(stack_phys) = lds.init_stack_end_phys(kernel_image).and_then(|stack_end| {
            stack_end
                .checked_sub(PT_SIZE_ON_STACK)
                .filter(|stack| *stack == head_sp)
        }) else {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        };

        if stack_phys < lds.init_stack_start() || stack_phys >= lds.init_stack_end() {
            return failed_condition(
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
            return failed_condition(
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

#[repr(C)]
pub struct InitTaskStorage {
    reserved: usize,
}

#[unsafe(no_mangle)]
pub static init_task_storage: InitTaskStorage = InitTaskStorage { reserved: 0 };
