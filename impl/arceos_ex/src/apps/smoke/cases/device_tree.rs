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

    let Some(root) = device_tree.root() else {
        printk::write_str("device tree root node ref failed\n");
        return SmokeResult::Failed;
    };
    if root.name() != b"/" || root.parent().is_some() {
        printk::write_str("device tree root node content invalid\n");
        return SmokeResult::Failed;
    }

    let Some(cpus) = device_tree.find_node(b"/cpus") else {
        printk::write_str("device tree /cpus node ref failed\n");
        return SmokeResult::Failed;
    };
    if cpus.name() != b"cpus" || cpus.parent().map(|node| node.name()) != Some(b"/".as_slice()) {
        printk::write_str("device tree /cpus parent or name invalid\n");
        return SmokeResult::Failed;
    }
    if cpus.children().next().is_none() {
        printk::write_str("device tree /cpus has no cpu child\n");
        return SmokeResult::Failed;
    }
    if cpus.property(b"#address-cells").is_none() {
        printk::write_str("device tree /cpus address-cells missing\n");
        return SmokeResult::Failed;
    }

    let Some(chosen) = device_tree.find_node(b"/chosen") else {
        printk::write_str("device tree /chosen node ref failed\n");
        return SmokeResult::Failed;
    };
    let Some(bootargs) = chosen.property(b"bootargs") else {
        printk::write_str("device tree /chosen bootargs missing\n");
        return SmokeResult::Failed;
    };
    if !contains(bootargs.raw_value(), b"earlycon=sbi") {
        printk::write_str("device tree bootargs value invalid\n");
        return SmokeResult::Failed;
    }

    let mut root_children = 0usize;
    for child in root.children() {
        if child.name().is_empty() {
            printk::write_str("device tree child has empty name\n");
            return SmokeResult::Failed;
        }
        root_children += 1;
    }
    if root_children == 0 {
        printk::write_str("device tree root has no children\n");
        return SmokeResult::Failed;
    }

    let mut chosen_properties = 0usize;
    for property in chosen.properties() {
        if property.name().is_empty() {
            printk::write_str("device tree property has empty name\n");
            return SmokeResult::Failed;
        }
        chosen_properties += 1;
    }
    if chosen_properties == 0 {
        printk::write_str("device tree /chosen has no properties\n");
        return SmokeResult::Failed;
    }

    printk::write_fmt(format_args!(
        "nodes={} properties={} root_children={} storage={:#x}..{:#x}\n",
        device_tree.node_count(),
        device_tree.property_count(),
        root_children,
        storage.start(),
        storage.end()
    ));
    SmokeResult::Passed
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() || needle.len() > haystack.len() {
        return false;
    }

    let mut index = 0usize;
    while index + needle.len() <= haystack.len() {
        if &haystack[index..index + needle.len()] == needle {
            return true;
        }
        index += 1;
    }
    false
}
