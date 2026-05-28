use crate::{
    apps::smoke::SmokeResult,
    context::context,
    objects::{printk, state::State},
};

pub fn run() -> SmokeResult {
    let ctx = context();
    let device_tree = &ctx.device_tree;
    let storage = device_tree.storage_range();

    if device_tree.state() != State::Ready {
        printk::write_str("device tree is not ready\n");
        return SmokeResult::Failed;
    }
    if storage.size() == 0 || storage.start() >= storage.end() {
        printk::write_str("device tree storage range invalid\n");
        return SmokeResult::Failed;
    }
    if device_tree.node_count() == 0 || device_tree.property_count() == 0 {
        printk::write_str("device tree has no nodes or properties\n");
        return SmokeResult::Failed;
    }
    if device_tree.find_path(b"/").is_none() {
        printk::write_str("device tree root lookup failed\n");
        return SmokeResult::Failed;
    }
    if device_tree.find_path(b"/cpus").is_none() {
        printk::write_str("device tree /cpus lookup failed\n");
        return SmokeResult::Failed;
    }
    if device_tree.find_path(b"/chosen").is_none() {
        printk::write_str("device tree /chosen lookup failed\n");
        return SmokeResult::Failed;
    }

    printk::write_fmt(format_args!(
        "nodes={} properties={} storage={:#x}..{:#x}\n",
        device_tree.node_count(),
        device_tree.property_count(),
        storage.start(),
        storage.end()
    ));
    SmokeResult::Passed
}
