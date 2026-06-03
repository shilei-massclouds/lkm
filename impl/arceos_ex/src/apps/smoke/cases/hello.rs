use crate::{apps::smoke::SmokeResult, objects::printk};

pub fn run() -> SmokeResult {
    printk::write_str("Hello, smoke!\n");
    #[cfg(not(checkpoint_handler_smoke))]
    earlycon::drain_printk();
    SmokeResult::Passed
}

#[cfg(not(checkpoint_handler_smoke))]
use crate::objects::earlycon;
