pub mod csr;
pub mod sbi;
pub mod task_switch;

pub const SUPERVISOR_TIMER_IRQ: usize = 5;
pub const SUPERVISOR_EXTERNAL_IRQ: usize = 9;
