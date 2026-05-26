use crate::{
    objects::{
        boot_args::BootArgs,
        entry_prelude::EntryPreludeObjects,
        state::EventResult,
    },
    trace::{self, Checkpoint},
};

pub fn setup_after_head_prefix(boot_args: &BootArgs, objects: &mut EntryPreludeObjects) -> ! {
    require(objects.adopt_head_prefix(boot_args));
    require(objects.event_stream_preset());
    require(objects.vm_preset(boot_args));
    objects.vm_setup()
}

pub fn after_vm_setup(objects: &mut EntryPreludeObjects) -> EventResult {
    let result = objects.after_vm_setup();
    if result.is_success() {
        trace::checkpoint(Checkpoint::EntryPreludePhaseReady);
    }
    result
}

fn require(result: EventResult) {
    if !result.is_success() {
        crate::arch::riscv64::sbi::putstr("arceos_ex entry prelude event failed\n");
        crate::arch::riscv64::sbi::system_shutdown()
    }
}
