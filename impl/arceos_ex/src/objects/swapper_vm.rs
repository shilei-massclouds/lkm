use crate::{arch::riscv64::csr, checkpoint::Checkpoint};

use super::{
    config::Config,
    kernel_image::KernelImage,
    lds::Lds,
    memblock::MemBlock,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
    static_objects::StaticObjects,
};

pub struct SwapperVm {
    lifecycle: Lifecycle,
    translation_sync_complete: bool,
    strict_kernel_rwx_boundary_deferred: bool,
    final_permissions_not_split_yet: bool,
}

impl SwapperVm {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            translation_sync_complete: false,
            strict_kernel_rwx_boundary_deferred: false,
            final_permissions_not_split_yet: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn translation_sync_complete(&self) -> bool {
        self.translation_sync_complete
    }

    pub const fn strict_kernel_rwx_boundary_deferred(&self) -> bool {
        self.strict_kernel_rwx_boundary_deferred
    }

    pub const fn final_permissions_not_split_yet(&self) -> bool {
        self.final_permissions_not_split_yet
    }

    pub fn setup(
        &mut self,
        config: &Config,
        static_objects: &mut StaticObjects,
        lds: &Lds,
        kernel_image: &KernelImage,
        memblock: &MemBlock,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || config.state() != State::Online
            || static_objects.state() != State::Online
            || !static_objects.storage_ready(config)
            || lds.state() != State::Online
            || kernel_image.state() != State::Online
            || memblock.state() != State::Ready
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        if !static_objects.build_swapper_pg_dir(
            config,
            kernel_image,
            lds.kernel_start(),
            lds.kernel_end(),
            memblock,
        ) {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }
        self.strict_kernel_rwx_boundary_deferred = true;
        self.final_permissions_not_split_yet = true;

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::SwapperVmReady,
        )
    }

    pub fn enable(
        &mut self,
        static_objects: &StaticObjects,
        kernel_image: &KernelImage,
    ) -> EventResult {
        if self.lifecycle.state() != State::Ready {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }

        let Some(satp) = static_objects.swapper_satp(kernel_image) else {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        };
        csr::write_satp(satp);
        csr::sfence_vma();
        self.translation_sync_complete = true;

        self.lifecycle.transition(
            LifecycleEvent::Enable,
            State::Ready,
            State::Online,
            Checkpoint::SwapperVmOnline,
        )
    }
}
