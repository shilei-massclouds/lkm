//! Interrupt leaf-phase namespace. Lifecycle ownership belongs to `BootInitFlow`.

pub mod irq_open_prepare;
pub mod irq_time_init;
pub mod local_irq_enable;
pub mod process_prepare;
