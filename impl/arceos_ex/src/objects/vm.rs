use crate::{arch::riscv64::csr, trace::Checkpoint};

use super::{
    boot_args::BootArgs,
    config::Config,
    early_vm::EarlyVm,
    entry_prelude::{KernelImage, Lds},
    fix_map::FixMap,
    raw_dtb::RawDtb,
    state::{EventResult, Lifecycle, LifecycleEvent, State},
    static_objects::StaticObjects,
    swapper_vm::SwapperVm,
    trampoline_vm::TrampolineVm,
};

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
            return EventResult::failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        let result = self.trampoline_vm.setup(config, static_objects, lds);
        if !result.is_success() {
            return result;
        }

        let result = self.early_vm.preset(config, boot_args, raw_dtb, fix_map);
        if !result.is_success() {
            return result;
        }

        let result =
            self.early_vm
                .setup(config, static_objects, lds, kernel_image, raw_dtb, fix_map);
        if !result.is_success() {
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
        config: &Config,
        static_objects: &StaticObjects,
        lds: &Lds,
        kernel_image: &mut KernelImage,
    ) -> ! {
        if self.lifecycle.state() != State::Prepared
            || self.trampoline_vm.state() != State::Ready
            || self.early_vm.state() != State::Ready
        {
            crate::arch::riscv64::sbi::system_shutdown();
        }

        let Some(trampoline_satp) = static_objects.trampoline_satp(config) else {
            crate::arch::riscv64::sbi::system_shutdown();
        };
        let Some(early_satp) = static_objects.early_satp(config) else {
            crate::arch::riscv64::sbi::system_shutdown();
        };
        let Some(stack_virt) = lds.init_stack_end_phys(config).and_then(|stack_end| {
            stack_end
                .checked_sub(super::entry_prelude::PT_SIZE_ON_STACK)
                .and_then(|stack| config.phys_to_link(stack))
        }) else {
            crate::arch::riscv64::sbi::system_shutdown();
        };
        let Some(gp_virt) = config.runtime_to_link(lds.global_pointer()) else {
            crate::arch::riscv64::sbi::system_shutdown();
        };
        let Some(continuation_virt) =
            config.runtime_to_link(vm_setup_continuation as usize)
        else {
            crate::arch::riscv64::sbi::system_shutdown();
        };

        let Some(vm_virt) = config.runtime_to_link(self as *mut Vm as usize) else {
            crate::arch::riscv64::sbi::system_shutdown();
        };
        let Some(kernel_image_virt) =
            config.runtime_to_link(kernel_image as *mut KernelImage as usize)
        else {
            crate::arch::riscv64::sbi::system_shutdown();
        };
        let Some(lds_virt) = config.runtime_to_link(lds as *const Lds as usize) else {
            crate::arch::riscv64::sbi::system_shutdown();
        };

        let context = VmSetupContext {
            vm: vm_virt as *mut Vm,
            kernel_image: kernel_image_virt as *mut KernelImage,
            lds: lds_virt as *const Lds,
        };
        set_vm_setup_context(config, context);

        unsafe {
            csr::switch_to_early_vm(
                trampoline_satp,
                early_satp,
                stack_virt,
                gp_virt,
                continuation_virt,
                config.kernel_virt_offset(),
            )
        }
    }

    fn finish_setup_after_switch(&mut self, kernel_image: &mut KernelImage, lds: &Lds) {
        let result = self.trampoline_vm.enable(kernel_image);
        if !result.is_success() {
            crate::arch::riscv64::sbi::system_shutdown();
        }

        let result = self.early_vm.enable(&self.trampoline_vm);
        if !result.is_success() {
            crate::arch::riscv64::sbi::system_shutdown();
        }

        let result = self.trampoline_vm.cleanup(&self.early_vm);
        if !result.is_success() {
            crate::arch::riscv64::sbi::system_shutdown();
        }

        let result = kernel_image.enable(lds);
        if !result.is_success() {
            crate::arch::riscv64::sbi::system_shutdown();
        }

        let result = self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::VmReady,
        );
        if !result.is_success() {
            crate::arch::riscv64::sbi::system_shutdown();
        }
    }

    pub fn enable(
        &mut self,
        config: &Config,
        static_objects: &mut StaticObjects,
        lds: &Lds,
        kernel_image: &KernelImage,
        memblock: &super::entry_successor::MemBlock,
    ) -> EventResult {
        if self.lifecycle.state() != State::Ready || self.early_vm.state() != State::Online {
            return EventResult::failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }

        let result = self
            .swapper_vm
            .setup(config, static_objects, lds, kernel_image, memblock);
        if !result.is_success() {
            return result;
        }

        let result = self.swapper_vm.enable(config, static_objects);
        if !result.is_success() {
            return result;
        }

        let result = self.early_vm.cleanup(&self.swapper_vm);
        if !result.is_success() {
            return result;
        }

        self.lifecycle.transition(
            LifecycleEvent::Enable,
            State::Ready,
            State::Online,
            Checkpoint::VmOnline,
        )
    }
}

#[derive(Clone, Copy)]
struct VmSetupContext {
    vm: *mut Vm,
    kernel_image: *mut KernelImage,
    lds: *const Lds,
}

static mut VM_SETUP_CONTEXT: VmSetupContext = VmSetupContext {
    vm: core::ptr::null_mut(),
    kernel_image: core::ptr::null_mut(),
    lds: core::ptr::null(),
};

fn set_vm_setup_context(config: &Config, context: VmSetupContext) {
    let Some(context_addr) = config.runtime_to_phys(core::ptr::addr_of!(VM_SETUP_CONTEXT) as usize)
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
    if context.vm.is_null() || context.kernel_image.is_null() || context.lds.is_null() {
        crate::arch::riscv64::sbi::system_shutdown();
    }

    let vm = unsafe { &mut *context.vm };
    let kernel_image = unsafe { &mut *context.kernel_image };
    let lds = unsafe { &*context.lds };
    vm.finish_setup_after_switch(kernel_image, lds);
    crate::phases::boot::after_vm_setup_continuation()
}
