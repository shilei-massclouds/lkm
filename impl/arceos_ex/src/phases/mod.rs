pub mod boot;
pub mod interrupt;
pub mod payload;
pub mod prepare;
pub mod smp_runtime;
pub mod state;
pub mod up_multitask;

use crate::objects::state::{EventError, EventResult};

pub fn shutdown_on_error(result: EventResult, message: &str) {
    if let Err(error) = result {
        crate::arch::riscv64::sbi::putstr(message);
        print_event_error(error);
        crate::arch::riscv64::sbi::system_shutdown()
    }
}

fn print_event_error(error: EventError) {
    use crate::arch::riscv64::sbi;

    sbi::putstr("error=");
    sbi::putchar(error.error_code());
    sbi::putstr(" event=");
    sbi::putchar(error.event_code());
    sbi::putstr(" actual=");
    sbi::putchar(error.actual_state_code());
    sbi::putstr(" expected=");
    sbi::putchar(error.expected_state_code());
    sbi::putstr(" target=");
    sbi::putchar(error.target_state_code());
    sbi::putchar(b'\n');
}
