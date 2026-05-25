use crate::{
    objects::{boot_args::BootArgs, earlycon, printk, state::EventResult},
    trace,
};

pub fn run(boot_args: &BootArgs) {
    let _ = boot_args.state();
    let _ = boot_args.boot_hartid();
    let _ = boot_args.dtb_pa();
    entry_prelude_phase_setup();
    entry_successor_phase_setup();
}

fn entry_prelude_phase_setup() {
    trace::checkpoint(b'A');
}

fn entry_successor_phase_setup() {
    trace::checkpoint(b'P');
    require(printk::preset());
    printk::write_str("arceos_ex object kernel\n");

    require(earlycon::preset());
    require(earlycon::setup());
    require(earlycon::enable());
    earlycon::drain_printk();
    trace::checkpoint(b'R');
}

fn require(result: EventResult) {
    if !result.is_success() {
        printk::write_str("arceos_ex boot event failed\n");
        earlycon::drain_printk();
        crate::arch::riscv64::sbi::system_shutdown()
    }
}
