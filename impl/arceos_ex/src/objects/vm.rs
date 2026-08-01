use crate::{arch::riscv64::csr, checkpoint::Checkpoint};

use super::{
    boot_args::BootArgs,
    config::Config,
    cpu::{Cpu, TranslationActivationKind, TranslationActivationTrace, TranslationController},
    early_vm::EarlyVm,
    fix_map::FixMap,
    kernel_addr_space::KernelAddrSpace,
    kernel_image::KernelImage,
    lds::Lds,
    physical_direct::PhysicalDirect,
    raw_dtb::RawDtb,
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
    static_objects::StaticObjects,
    swapper_vm::SwapperVm,
    trampoline_vm::TrampolineVm,
    vm_setup::{self, VmSetupContinuation},
};

pub struct Vm {
    lifecycle: Lifecycle,
    physical_direct: PhysicalDirect,
    trampoline_vm: TrampolineVm,
    early_vm: EarlyVm,
    swapper_vm: SwapperVm,
    early_boot_alternatives_deferred: bool,
    early_boot_alternatives_mmu_off_boundary_preserved: bool,
    kernel_addr_space_online_observed: bool,
}

impl Vm {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            physical_direct: PhysicalDirect::new(),
            trampoline_vm: TrampolineVm::new(),
            early_vm: EarlyVm::new(),
            swapper_vm: SwapperVm::new(),
            early_boot_alternatives_deferred: false,
            early_boot_alternatives_mmu_off_boundary_preserved: false,
            kernel_addr_space_online_observed: false,
        }
    }

    #[allow(dead_code)]
    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    // VM preset is the specified pre-MMU handoff across static, image, DT and fixmap objects.
    #[allow(clippy::too_many_arguments)]
    pub fn preset(
        &mut self,
        config: &Config,
        static_objects: &mut StaticObjects,
        lds: &Lds,
        kernel_image: &KernelImage,
        boot_args: &BootArgs,
        raw_dtb: &mut RawDtb,
        fix_map: &mut FixMap,
        kernel_addr_space: &mut KernelAddrSpace,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || self.trampoline_vm.state() != State::Base
            || self.early_vm.state() != State::Base
            || kernel_addr_space.state() != State::Base
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        kernel_addr_space.preset(config, lds, kernel_image)?;

        self.trampoline_vm
            .setup(config, static_objects, lds, kernel_image)?;

        self.early_vm.preset(config, boot_args, raw_dtb, fix_map)?;

        kernel_addr_space.setup(config, fix_map)?;

        self.early_vm
            .setup(config, static_objects, lds, kernel_image, raw_dtb, fix_map)?;

        self.early_boot_alternatives_deferred = true;
        self.early_boot_alternatives_mmu_off_boundary_preserved = true;

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
        boot_cpu: &Cpu,
        after_switch: VmSetupContinuation,
    ) -> ! {
        if self.lifecycle.state() != State::Prepared
            || self.trampoline_vm.state() != State::Ready
            || self.early_vm.state() != State::Ready
            || self.physical_direct.state() != State::Ready
            || boot_cpu.active_translation_controller()
                != Ok(Some(TranslationController::PhysicalDirect))
            || csr::read_satp() != 0
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
        if !self
            .trampoline_vm
            .translation_state_mapped(boot_cpu, kernel_image)
        {
            crate::arch::riscv64::sbi::system_shutdown();
        }
        let Some(translation_state_phys) =
            kernel_image.runtime_to_phys(boot_cpu.translation_state_storage() as usize)
        else {
            crate::arch::riscv64::sbi::system_shutdown();
        };
        let Some(context) =
            vm_setup::VmSetupContext::new(self, kernel_image, lds, boot_cpu, after_switch)
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
                translation_state_phys,
            )
        }
    }

    pub(super) fn finish_setup_after_switch(
        &mut self,
        kernel_image: &mut KernelImage,
        lds: &Lds,
        boot_cpu: &Cpu,
    ) {
        crate::checkpoint::enable_post_vm_checkpoints();

        let expected_chain =
            boot_early_translation_chain(self.trampoline_vm.satp(), self.early_vm.satp());
        if !boot_cpu.verify_translation_chain(&expected_chain, csr::read_satp())
            || !self.physical_direct.complete_arch_activation_on(boot_cpu)
            || !self.trampoline_vm.complete_arch_activation_on(boot_cpu)
        {
            crate::arch::riscv64::sbi::system_shutdown();
        }
        crate::checkpoint::checkpoint(Checkpoint::TrampolineVmActivatedOnCpu);

        if !self.early_vm.complete_arch_activation_on(boot_cpu) {
            crate::arch::riscv64::sbi::system_shutdown();
        }
        crate::checkpoint::checkpoint(Checkpoint::EarlyVmActivatedOnCpu);

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

    #[allow(clippy::too_many_arguments)]
    pub fn enable(
        &mut self,
        config: &Config,
        static_objects: &mut StaticObjects,
        lds: &Lds,
        kernel_image: &KernelImage,
        memblock: &super::memblock::MemBlock,
        boot_cpu: &Cpu,
        kernel_addr_space: &mut KernelAddrSpace,
    ) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || self.early_vm.state() != State::Ready
            || !self.early_vm.current_on_cpu(boot_cpu)
            || kernel_addr_space.state() != State::Ready
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }

        self.swapper_vm
            .setup(config, static_objects, lds, kernel_image, memblock)?;

        kernel_addr_space.enable(&self.swapper_vm)?;
        self.kernel_addr_space_online_observed = true;
        self.swapper_vm
            .activate_on_cpu(boot_cpu, self.early_vm.satp(), kernel_addr_space)?;

        self.lifecycle.transition(
            LifecycleEvent::Enable,
            State::Ready,
            State::Online,
            Checkpoint::VmOnline,
        )
    }

    pub(crate) fn adopt_head_physical_on_cpu(&self, cpu: &Cpu) -> EventResult {
        self.physical_direct.adopt_head_activation_on(cpu)
    }

    pub fn entry_prelude_ready(&self) -> bool {
        self.physical_direct.state() == State::Ready
            && self.trampoline_vm.state() == State::Ready
            && self.early_vm.state() == State::Ready
            && self.early_boot_alternatives_deferred
            && self.early_boot_alternatives_mmu_off_boundary_preserved
    }

    pub fn entry_prelude_ready_for(&self, cpu: &Cpu) -> bool {
        self.lifecycle.state() == State::Ready
            && self.entry_prelude_ready()
            && self.physical_direct.activation_complete_on(cpu)
            && self.trampoline_vm.translation_sync_complete(cpu)
            && self.early_vm.current_on_cpu(cpu)
    }

    pub fn boot_init_setup_ready(&self) -> bool {
        self.lifecycle.state() == State::Online
            && self.physical_direct.state() == State::Ready
            && self.trampoline_vm.state() == State::Ready
            && self.early_vm.state() == State::Ready
            && self.swapper_vm.state() == State::Ready
            && self.kernel_addr_space_online_observed
            && self.swapper_vm.strict_kernel_rwx_boundary_deferred()
            && self.swapper_vm.final_permissions_not_split_yet()
    }

    pub fn boot_init_setup_ready_for(&self, cpu: &Cpu) -> bool {
        self.boot_init_setup_ready() && self.swapper_vm.current_on_cpu(cpu)
    }

    pub fn complete_ap_translation_chain(&self, cpu: &Cpu, live_satp: usize) -> bool {
        let expected_chain =
            ap_translation_chain(self.trampoline_vm.satp(), self.swapper_vm.satp());
        self.boot_init_setup_ready()
            && cpu.verify_translation_chain(&expected_chain, live_satp)
            && self.physical_direct.complete_arch_activation_on(cpu)
            && self.trampoline_vm.complete_arch_activation_on(cpu)
            && self
                .swapper_vm
                .complete_arch_activation_on(cpu, TranslationController::TrampolineVm)
    }

    pub fn ap_translation_ready(&self, cpu: &Cpu) -> bool {
        let expected_chain =
            ap_translation_chain(self.trampoline_vm.satp(), self.swapper_vm.satp());
        self.boot_init_setup_ready()
            && self.physical_direct.activation_complete_on(cpu)
            && self.trampoline_vm.translation_sync_complete(cpu)
            && self.swapper_vm.activation_complete_on(cpu)
            && cpu.translation_chain_matches(&expected_chain)
    }

    pub const fn swapper_vm(&self) -> &SwapperVm {
        &self.swapper_vm
    }

    pub const fn early_vm_satp(&self) -> usize {
        self.early_vm.satp()
    }

    pub const fn trampoline_vm(&self) -> &TrampolineVm {
        &self.trampoline_vm
    }
}

fn boot_early_translation_chain(
    trampoline_satp: usize,
    early_satp: usize,
) -> [TranslationActivationTrace; 3] {
    [
        TranslationActivationTrace::completed(
            TranslationActivationKind::InitialActivation,
            None,
            TranslationController::PhysicalDirect,
            0,
            1,
        ),
        TranslationActivationTrace::completed(
            TranslationActivationKind::Handoff,
            Some(TranslationController::PhysicalDirect),
            TranslationController::TrampolineVm,
            trampoline_satp,
            2,
        ),
        TranslationActivationTrace::completed(
            TranslationActivationKind::Handoff,
            Some(TranslationController::TrampolineVm),
            TranslationController::EarlyVm,
            early_satp,
            3,
        ),
    ]
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
pub(crate) fn boot_translation_chain(
    trampoline_satp: usize,
    early_satp: usize,
    swapper_satp: usize,
) -> [TranslationActivationTrace; 4] {
    let early = boot_early_translation_chain(trampoline_satp, early_satp);
    [
        early[0],
        early[1],
        early[2],
        TranslationActivationTrace::completed(
            TranslationActivationKind::Handoff,
            Some(TranslationController::EarlyVm),
            TranslationController::SwapperVm,
            swapper_satp,
            4,
        ),
    ]
}

pub(crate) fn ap_translation_chain(
    trampoline_satp: usize,
    swapper_satp: usize,
) -> [TranslationActivationTrace; 3] {
    [
        TranslationActivationTrace::completed(
            TranslationActivationKind::InitialActivation,
            None,
            TranslationController::PhysicalDirect,
            0,
            1,
        ),
        TranslationActivationTrace::completed(
            TranslationActivationKind::Handoff,
            Some(TranslationController::PhysicalDirect),
            TranslationController::TrampolineVm,
            trampoline_satp,
            2,
        ),
        TranslationActivationTrace::completed(
            TranslationActivationKind::Handoff,
            Some(TranslationController::TrampolineVm),
            TranslationController::SwapperVm,
            swapper_satp,
            3,
        ),
    ]
}
