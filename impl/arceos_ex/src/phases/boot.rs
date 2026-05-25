use crate::{
    objects::{boot_args::BootArgs, entry_prelude::EntryPreludeObjects, state::EventResult},
    trace::{self, Checkpoint},
};

pub fn run(boot_args: &BootArgs) {
    let _ = boot_args.state();
    let _ = boot_args.boot_hartid();
    let _ = boot_args.dtb_pa();
    let mut entry_prelude = EntryPreludeObjects::new();
    entry_prelude_phase_setup(boot_args, &mut entry_prelude);
    smoke_output_after_entry_prelude_foundation();
}

fn entry_prelude_phase_setup(boot_args: &BootArgs, objects: &mut EntryPreludeObjects) {
    trace::checkpoint(Checkpoint::EntryPreludePhaseStarted);
    require(objects.interrupt_stream_preset());
    require(objects.kernel_image_preset());
    require(objects.root_stream_preset());
    require(objects.kernel_image_setup());
    require(objects.cpu_group_preset(boot_args));
    require(objects.init_task_preset());
    require(objects.init_stack_preset());
    require(objects.event_stream_preset());
    require(objects.raw_dtb_preset(boot_args));
    require(objects.raw_dtb_setup());
    require(objects.fix_map_preset());
    trace::checkpoint(Checkpoint::EntryPreludeFoundationReady);
}

fn smoke_output_after_entry_prelude_foundation() {
    crate::arch::riscv64::sbi::putstr("arceos_ex object kernel\n");
}

fn require(result: EventResult) {
    if !result.is_success() {
        crate::arch::riscv64::sbi::putstr("arceos_ex boot event failed\n");
        crate::arch::riscv64::sbi::system_shutdown()
    }
}
