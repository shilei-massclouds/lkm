use crate::{
    objects::{
        boot_args::BootArgs,
        entry_prelude::EntryPreludeObjects,
        entry_successor::EntrySuccessorObjects,
        state::EventResult,
    },
    trace::{self, Checkpoint},
};

static mut ENTRY_PRELUDE: EntryPreludeObjects = EntryPreludeObjects::new();
static mut ENTRY_SUCCESSOR: EntrySuccessorObjects = EntrySuccessorObjects::new();

pub fn run(boot_args: &BootArgs) -> ! {
    let _ = boot_args.state();
    let _ = boot_args.boot_hartid();
    let _ = boot_args.dtb_pa();
    let entry_prelude = entry_prelude_objects();
    entry_prelude_phase_setup(boot_args, entry_prelude);
}

fn entry_prelude_phase_setup(boot_args: &BootArgs, objects: &mut EntryPreludeObjects) -> ! {
    trace::checkpoint(Checkpoint::EntryPreludePhaseStarted);
    require(objects.interrupt_stream_preset());
    require(objects.kernel_image_preset());
    require(objects.root_stream_preset());
    require(objects.kernel_image_setup());
    require(objects.cpu_group_preset(boot_args));
    require(objects.init_task_preset());
    require(objects.init_stack_preset());
    require(objects.event_stream_preset());
    require(objects.vm_preset(boot_args));
    objects.vm_setup()
}

pub fn after_vm_setup_continuation() -> ! {
    let entry_prelude = entry_prelude_objects();
    require(entry_prelude.after_vm_setup());
    trace::checkpoint(Checkpoint::EntryPreludePhaseReady);
    let entry_successor = entry_successor_objects();
    require(entry_successor.setup(entry_prelude));
    crate::app_main();
    crate::arch::riscv64::sbi::system_shutdown()
}

fn require(result: EventResult) {
    if !result.is_success() {
        crate::arch::riscv64::sbi::putstr("arceos_ex boot event failed\n");
        crate::arch::riscv64::sbi::system_shutdown()
    }
}

fn entry_prelude_objects() -> &'static mut EntryPreludeObjects {
    // SAFETY: the boot path is single-hart and system-exclusive here. This
    // static carrier keeps object state alive across Vm.Setup, which does not
    // return to its physical-address caller.
    unsafe { &mut *core::ptr::addr_of_mut!(ENTRY_PRELUDE) }
}

fn entry_successor_objects() -> &'static mut EntrySuccessorObjects {
    // SAFETY: same boot-exclusive context as ENTRY_PRELUDE.
    unsafe { &mut *core::ptr::addr_of_mut!(ENTRY_SUCCESSOR) }
}
