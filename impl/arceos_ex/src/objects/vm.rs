use crate::{arch::riscv64::csr, trace::Checkpoint};

use super::{
    boot_args::BootArgs,
    config::Config,
    early_vm::EarlyVm,
    fix_map::FixMap,
    kernel_image::KernelImage,
    lds::Lds,
    raw_dtb::RawDtb,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
    static_objects::StaticObjects,
    swapper_vm::SwapperVm,
    trampoline_vm::TrampolineVm,
    vm_setup::{self, VmSetupContinuation},
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
        let Some(continuation_virt) = vm_setup::continuation_addr(kernel_image) else {
            crate::arch::riscv64::sbi::system_shutdown();
        };
        let Some(context) = vm_setup::VmSetupContext::new(self, kernel_image, lds, after_switch)
        else {
            crate::arch::riscv64::sbi::system_shutdown();
        };
        vm_setup::set_context(kernel_image, context);

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

    pub(super) fn finish_setup_after_switch(&mut self, kernel_image: &mut KernelImage, lds: &Lds) {
        crate::trace::enable_post_vm_checkpoints();

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

    pub const fn swapper_vm(&self) -> &SwapperVm {
        &self.swapper_vm
    }
}
