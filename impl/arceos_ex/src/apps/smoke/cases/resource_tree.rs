use crate::{
    apps::smoke::SmokeResult,
    context::context,
    objects::{
        printk,
        resource_tree::{ResourceKind, ResourceRef},
        state::State,
    },
};

pub fn run() -> SmokeResult {
    let ctx = context();
    let resource_tree = &ctx.resource_tree;

    if resource_tree.state() != State::Ready {
        printk::write_str("resource tree is not ready\n");
        return SmokeResult::Failed;
    }

    let Some(root) = resource_tree.root() else {
        printk::write_str("resource tree root missing\n");
        return SmokeResult::Failed;
    };
    if root.parent().is_some() || root.children().next().is_none() {
        printk::write_str("resource tree root relationship invalid\n");
        return SmokeResult::Failed;
    }

    let Some(kernel) = resource_tree.find_first(ResourceKind::KernelImage) else {
        printk::write_str("kernel image resource missing\n");
        return SmokeResult::Failed;
    };
    if kernel.parent().map(|parent| parent.kind()) != Some(ResourceKind::SystemRam) {
        printk::write_str("kernel image resource parent invalid\n");
        return SmokeResult::Failed;
    }

    let mut kernel_segment_count = 0usize;
    for kind in [
        ResourceKind::KernelCode,
        ResourceKind::KernelRodata,
        ResourceKind::KernelData,
        ResourceKind::KernelBss,
    ] {
        let Some(segment) = resource_tree.find_first(kind) else {
            printk::write_str("kernel segment resource missing\n");
            return SmokeResult::Failed;
        };
        if segment.parent().map(|parent| parent.kind()) != Some(ResourceKind::KernelImage)
            || !contains(kernel, segment)
        {
            printk::write_str("kernel segment relationship invalid\n");
            return SmokeResult::Failed;
        }
        kernel_segment_count += 1;
    }

    let mut system_ram_count = 0usize;
    let mut system_ram_bytes = 0usize;
    for system_ram in resource_tree.resources(ResourceKind::SystemRam) {
        if system_ram.range().start() >= system_ram.range().end() {
            printk::write_str("system RAM resource range invalid\n");
            return SmokeResult::Failed;
        }
        system_ram_count += 1;
        system_ram_bytes += system_ram.range().size();
    }
    if system_ram_count == 0 || system_ram_bytes == 0 {
        printk::write_str("system RAM resource missing\n");
        return SmokeResult::Failed;
    }

    let mut reserved_count = 0usize;
    for reserved in resource_tree.resources(ResourceKind::Reserved) {
        let Some(parent) = reserved.parent() else {
            printk::write_str("reserved resource has no parent\n");
            return SmokeResult::Failed;
        };
        if (parent.kind() != ResourceKind::Root && parent.kind() != ResourceKind::SystemRam)
            || !contains(parent, reserved)
        {
            printk::write_str("reserved resource parent invalid\n");
            return SmokeResult::Failed;
        }
        reserved_count += 1;
    }
    if reserved_count == 0 {
        printk::write_str("reserved resource missing\n");
        return SmokeResult::Failed;
    }

    printk::write_fmt(format_args!(
        "resources={} root=1 system_ram={} reserved={} kernel=1 segments={} kernel_name={} kernel={:#x}..{:#x}\n",
        resource_tree.resource_count(),
        system_ram_count,
        reserved_count,
        kernel_segment_count,
        kernel.kind().name(),
        kernel.range().start(),
        kernel.range().end()
    ));
    SmokeResult::Passed
}

fn contains(parent: ResourceRef<'_>, child: ResourceRef<'_>) -> bool {
    parent.range().start() <= child.range().start() && parent.range().end() >= child.range().end()
}
