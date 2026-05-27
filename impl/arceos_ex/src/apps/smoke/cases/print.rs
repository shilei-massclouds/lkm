use crate::{apps::smoke::SmokeResult, objects::printk};

pub fn run() -> SmokeResult {
    printk::write_fmt(format_args!(
        "formatted value={} hex={:#x}\n",
        42usize, 42usize
    ));
    SmokeResult::Passed
}
