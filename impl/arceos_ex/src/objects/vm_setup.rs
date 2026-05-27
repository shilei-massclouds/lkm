use super::{kernel_image::KernelImage, lds::Lds, vm::Vm};

pub type VmSetupContinuation = extern "C" fn() -> !;

#[derive(Clone, Copy)]
pub struct VmSetupContext {
    vm: *mut Vm,
    kernel_image: *mut KernelImage,
    lds: *const Lds,
    after_switch: usize,
}

impl VmSetupContext {
    pub fn new(
        vm: &mut Vm,
        kernel_image: &mut KernelImage,
        lds: &Lds,
        after_switch: VmSetupContinuation,
    ) -> Option<Self> {
        let vm_virt = kernel_image.runtime_to_link(vm as *mut Vm as usize)?;
        let kernel_image_addr = kernel_image as *mut KernelImage as usize;
        let kernel_image_virt = kernel_image.runtime_to_link(kernel_image_addr)?;
        let lds_virt = kernel_image.runtime_to_link(lds as *const Lds as usize)?;
        let after_switch_virt = kernel_image.runtime_to_link(after_switch as usize)?;

        Some(Self {
            vm: vm_virt as *mut Vm,
            kernel_image: kernel_image_virt as *mut KernelImage,
            lds: lds_virt as *const Lds,
            after_switch: after_switch_virt,
        })
    }
}

static mut VM_SETUP_CONTEXT: VmSetupContext = VmSetupContext {
    vm: core::ptr::null_mut(),
    kernel_image: core::ptr::null_mut(),
    lds: core::ptr::null(),
    after_switch: 0,
};

pub fn continuation_addr(kernel_image: &KernelImage) -> Option<usize> {
    kernel_image.runtime_to_link(vm_setup_continuation as usize)
}

pub fn set_context(kernel_image: &KernelImage, context: VmSetupContext) {
    let Some(context_addr) =
        kernel_image.runtime_to_phys(core::ptr::addr_of!(VM_SETUP_CONTEXT) as usize)
    else {
        crate::arch::riscv64::sbi::system_shutdown();
    };
    unsafe {
        core::ptr::write_volatile(context_addr as *mut VmSetupContext, context);
    }
}

#[unsafe(no_mangle)]
extern "C" fn vm_setup_continuation() -> ! {
    let context = unsafe { core::ptr::read_volatile(core::ptr::addr_of!(VM_SETUP_CONTEXT)) };
    if context.vm.is_null()
        || context.kernel_image.is_null()
        || context.lds.is_null()
        || context.after_switch == 0
    {
        crate::arch::riscv64::sbi::system_shutdown();
    }

    let vm = unsafe { &mut *context.vm };
    let kernel_image = unsafe { &mut *context.kernel_image };
    let lds = unsafe { &*context.lds };
    vm.finish_setup_after_switch(kernel_image, lds);
    let after_switch: VmSetupContinuation = unsafe { core::mem::transmute(context.after_switch) };
    after_switch()
}
