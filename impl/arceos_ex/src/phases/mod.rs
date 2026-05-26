pub mod boot;
pub mod prepare;
pub mod state;

use crate::objects::state::EventOutcome;

pub fn shutdown_on_error(result: EventOutcome, message: &str) {
    if result.is_err() {
        crate::arch::riscv64::sbi::putstr(message);
        crate::arch::riscv64::sbi::system_shutdown()
    }
}
