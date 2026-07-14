use crate::arch::riscv64::csr;

use super::{
    kernel_image::KernelImage,
    state::{Lifecycle, State},
};

unsafe extern "C" {
    fn _start();
    fn kernel_start();
    fn kernel_end();
    #[link_name = "__global_pointer$"]
    fn global_pointer();
    fn __head_text_start();
    fn __head_text_end();
    fn _stext();
    fn _etext();
    fn _srodata();
    fn _erodata();
    fn _sdata();
    fn _edata();
    fn __per_cpu_start();
    fn __per_cpu_end();
    fn __per_cpu_load();
    fn _sbss();
    fn _ebss();
    fn init_stack_start();
    fn init_stack_end();
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
        kernel_start as *const () as usize
    }

    pub fn kernel_end(&self) -> usize {
        kernel_end as *const () as usize
    }

    fn text_start(&self) -> usize {
        _start as *const () as usize
    }

    pub fn text_range(&self) -> (usize, usize) {
        (_stext as *const () as usize, _etext as *const () as usize)
    }

    pub fn rodata_range(&self) -> (usize, usize) {
        (
            _srodata as *const () as usize,
            _erodata as *const () as usize,
        )
    }

    pub fn data_range(&self) -> (usize, usize) {
        (_sdata as *const () as usize, _edata as *const () as usize)
    }

    pub fn per_cpu_start(&self) -> usize {
        __per_cpu_start as *const () as usize
    }

    pub fn per_cpu_end(&self) -> usize {
        __per_cpu_end as *const () as usize
    }

    pub fn per_cpu_load(&self) -> usize {
        __per_cpu_load as *const () as usize
    }

    pub fn bss_range(&self) -> (usize, usize) {
        (self.bss_start(), self.bss_end())
    }

    pub fn global_pointer(&self) -> usize {
        global_pointer as *const () as usize
    }

    pub fn current_global_pointer(&self, kernel_image: &KernelImage) -> Option<usize> {
        if csr::read_satp() == 0 {
            kernel_image.runtime_to_phys(self.global_pointer())
        } else {
            Some(self.global_pointer())
        }
    }

    fn head_text_start(&self) -> usize {
        __head_text_start as *const () as usize
    }

    fn head_text_end(&self) -> usize {
        __head_text_end as *const () as usize
    }

    fn bss_start(&self) -> usize {
        _sbss as *const () as usize
    }

    fn bss_end(&self) -> usize {
        _ebss as *const () as usize
    }

    pub fn init_stack_start(&self) -> usize {
        init_stack_start as *const () as usize
    }

    pub fn init_stack_end(&self) -> usize {
        init_stack_end as *const () as usize
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
            && self.per_cpu_static_image_layout_ready()
            && self.init_stack_end() > self.init_stack_start()
            && self.init_stack_start() >= kernel_start_addr
            && self.init_stack_end() <= kernel_end_addr
            && self.init_stack_start() & 0xfff == 0
            && self.init_stack_end() & 0xfff == 0
    }

    pub fn per_cpu_static_image_layout_ready(&self) -> bool {
        let kernel_start_addr = self.kernel_start();
        let kernel_end_addr = self.kernel_end();
        let per_cpu_start = self.per_cpu_start();
        let per_cpu_end = self.per_cpu_end();
        let per_cpu_load = self.per_cpu_load();

        if per_cpu_start == 0
            || per_cpu_end <= per_cpu_start
            || per_cpu_load == 0
            || per_cpu_start & 0xfff != 0
            || per_cpu_start < kernel_start_addr
            || per_cpu_end > kernel_end_addr
        {
            return false;
        }

        let static_size = per_cpu_end - per_cpu_start;
        let Some(load_end) = per_cpu_load.checked_add(static_size) else {
            return false;
        };

        per_cpu_load >= kernel_start_addr && load_end <= kernel_end_addr
    }

    pub(crate) fn bss_zeroed(&self, kernel_image: &KernelImage) -> bool {
        let Some(start) = kernel_image.runtime_to_phys(self.bss_start()) else {
            return false;
        };
        let len = self.bss_end() - self.bss_start();
        let bytes = unsafe { core::slice::from_raw_parts(start as *const u8, len) };
        bytes.iter().all(|byte| *byte == 0)
    }
}
