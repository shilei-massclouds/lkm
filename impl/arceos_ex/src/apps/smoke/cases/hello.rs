use crate::{apps::smoke::SmokeResult, objects::printk};

pub fn run() -> SmokeResult {
    printk::write_str("Hello, smoke!\n");
    SmokeResult::Passed
}
