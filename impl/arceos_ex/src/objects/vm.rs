use crate::{arch::riscv64::csr, trace::Checkpoint};

use super::{
    boot_args::BootArgs,
    config::Config,
    early_vm::EarlyVm,
    entry_prelude::Lds,
    fix_map::FixMap,
    kernel_image::KernelImage,
    raw_dtb::RawDtb,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
    static_objects::StaticObjects,
    swapper_vm::SwapperVm,
    trampoline_vm::TrampolineVm,
};

pub type VmSetupContinuation = extern "C" fn() -> !;

pub struct Vm {
    lifecycle: Lifecycle,
    trampoline_vm: TrampolineVm,
    early_vm: EarlyVm,
    swapper_vm: SwapperVm,
}

impl Vm {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            trampoline_vm: TrampolineVm::new(),
            early_vm: EarlyVm::new(),
            swapper_vm: SwapperVm::new(),
        }
    }

    #[allow(dead_code)]
    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub fn preset(
        &mut self,
        config: &Config,
        static_objects: &mut StaticObjects,
        lds: &Lds,
        kernel_image: &KernelImage,
        boot_args: &BootArgs,
        raw_dtb: &mut RawDtb,
        fix_map: &mut FixMap,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || self.trampoline_vm.state() != State::Base
            || self.early_vm.state() != State::Base
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        let result = self
            .trampoline_vm
            .setup(config, static_objects, lds, kernel_image);
        if result.is_err() {
            return result;
        }

        let result = self.early_vm.preset(config, boot_args, raw_dtb, fix_map);
        if result.is_err() {
            return result;
        }

        let result =
            self.early_vm
                .setup(config, static_objects, lds, kernel_image, raw_dtb, fix_map);
        if result.is_err() {
            return result;
        }

        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::VmPrepared,
        )
    }

    pub fn setup(
        &mut self,
        _config: &Config,
        static_objects: &StaticObjects,
        lds: &Lds,
        kernel_image: &mut KernelImage,
        after_switch: VmSetupContinuation,
    ) -> ! {
        if self.lifecycle.state() != State::Prepared
            || self.trampoline_vm.state() != State::Ready
            || self.early_vm.state() != State::Ready
        {
            crate::arch::riscv64::sbi::system_shutdown();
        }

        let Some(trampoline_satp) = static_objects.trampoline_satp(kernel_image) else {
            crate::arch::riscv64::sbi::system_shutdown();
        };
        let Some(early_satp) = static_objects.early_satp(kernel_image) else {
            crate::arch::riscv64::sbi::system_shutdown();
        };
        let Some(stack_virt) = lds.init_stack_end_phys(kernel_image).and_then(|stack_end| {
            stack_end
                .checked_sub(super::init_stack::PT_SIZE_ON_STACK)
                .and_then(|stack| kernel_image.phys_to_link(stack))
        }) else {
            crate::arch::riscv64::sbi::system_shutdown();
        };
        let Some(gp_virt) = kernel_image.runtime_to_link(lds.global_pointer()) else {
            crate::arch::riscv64::sbi::system_shutdown();
        };
        let Some(continuation_virt) = kernel_image.runtime_to_link(vm_setup_continuation as usize)
        else {
            crate::arch::riscv64::sbi::system_shutdown();
        };

        let Some(vm_virt) = kernel_image.runtime_to_link(self as *mut Vm as usize) else {
            crate::arch::riscv64::sbi::system_shutdown();
        };
        let kernel_image_addr = kernel_image as *mut KernelImage as usize;
        let Some(kernel_image_virt) = kernel_image.runtime_to_link(kernel_image_addr) else {
            crate::arch::riscv64::sbi::system_shutdown();
        };
        let Some(lds_virt) = kernel_image.runtime_to_link(lds as *const Lds as usize) else {
            crate::arch::riscv64::sbi::system_shutdown();
        };
        let Some(after_switch_virt) = kernel_image.runtime_to_link(after_switch as usize) else {
            crate::arch::riscv64::sbi::system_shutdown();
        };

        let context = VmSetupContext {
            vm: vm_virt as *mut Vm,
            kernel_image: kernel_image_virt as *mut KernelImage,
            lds: lds_virt as *const Lds,
            after_switch: after_switch_virt,
        };
        set_vm_setup_context(kernel_image, context);

        unsafe {
            csr::switch_to_early_vm(
                trampoline_satp,
                early_satp,
                stack_virt,
                gp_virt,
                continuation_virt,
                kernel_image.virt_offset(),
            )
        }
    }

    fn finish_setup_after_switch(&mut self, kernel_image: &mut KernelImage, lds: &Lds) {
        let result = self.trampoline_vm.enable(kernel_image);
        if result.is_err() {
            crate::arch::riscv64::sbi::system_shutdown();
        }

        let result = self.early_vm.enable(&self.trampoline_vm);
        if result.is_err() {
            crate::arch::riscv64::sbi::system_shutdown();
        }

        let result = self.trampoline_vm.cleanup(&self.early_vm);
        if result.is_err() {
            crate::arch::riscv64::sbi::system_shutdown();
        }

        let result = kernel_image.enable(lds);
        if result.is_err() {
            crate::arch::riscv64::sbi::system_shutdown();
        }

        let result = self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::VmReady,
        );
        if result.is_err() {
            crate::arch::riscv64::sbi::system_shutdown();
        }
    }

    pub fn enable(
        &mut self,
        config: &Config,
        static_objects: &mut StaticObjects,
        lds: &Lds,
        kernel_image: &KernelImage,
        memblock: &super::memblock::MemBlock,
    ) -> EventResult {
        if self.lifecycle.state() != State::Ready || self.early_vm.state() != State::Online {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }

        let result = self
            .swapper_vm
            .setup(config, static_objects, lds, kernel_image, memblock);
        if result.is_err() {
            return result;
        }

        let result = self.swapper_vm.enable(static_objects, kernel_image);
        if result.is_err() {
            return result;
        }

        let result = self.early_vm.cleanup(&self.swapper_vm);
        if result.is_err() {
            return result;
        }

        self.lifecycle.transition(
            LifecycleEvent::Enable,
            State::Ready,
            State::Online,
            Checkpoint::VmOnline,
        )
    }

    pub fn entry_prelude_ready(&self) -> bool {
        self.trampoline_vm.state() == State::Destroyed && self.early_vm.state() == State::Online
    }

    pub fn entry_successor_ready(&self) -> bool {
        self.lifecycle.state() == State::Online
            && self.swapper_vm.state() == State::Online
            && self.early_vm.state() == State::Destroyed
    }
}

#[derive(Clone, Copy)]
struct VmSetupContext {
    vm: *mut Vm,
    kernel_image: *mut KernelImage,
    lds: *const Lds,
    after_switch: usize,
}

static mut VM_SETUP_CONTEXT: VmSetupContext = VmSetupContext {
    vm: core::ptr::null_mut(),
    kernel_image: core::ptr::null_mut(),
    lds: core::ptr::null(),
    after_switch: 0,
};

fn set_vm_setup_context(kernel_image: &KernelImage, context: VmSetupContext) {
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
