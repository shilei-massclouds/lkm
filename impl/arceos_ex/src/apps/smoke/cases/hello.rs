use crate::{
    apps::smoke::SmokeResult,
    objects::{earlycon, printk},
};

pub fn run() -> SmokeResult {
    printk::write_str("Hello, smoke!\n");
    earlycon::drain_printk();
    SmokeResult::Passed
}
