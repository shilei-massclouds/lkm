use crate::{
    apps::smoke::{
        harness::{SmokeAssertions, SmokeScenario, SmokeSuite},
        SmokeResult,
    },
    context::context,
    objects::{
        devfs::DevFsNodeKind,
        state::State,
        vfs::{FileSystemKind, VfsInodeKind},
    },
};

pub fn run() -> SmokeResult {
    let mut suite = SmokeSuite::new();
    suite.scenario(&mut DevFsNodesScenario::new());
    suite.result()
}

struct DevFsNodesScenario;

impl DevFsNodesScenario {
    const fn new() -> Self {
        Self
    }
}

impl SmokeScenario for DevFsNodesScenario {
    fn name(&self) -> &'static str {
        "devfs.hwrng_and_block_nodes"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        let ctx = context();
        assertions.assert("devfs ready", ctx.devfs.state() == State::Ready);
        assertions.assert("devfs initialized", ctx.devfs.initialized());
        assertions.assert("devfs mounted", ctx.devfs.mounted());
        assertions.assert("devfs node count", ctx.devfs.node_count() == 2);
        assertions.assert("file ops deferred", ctx.devfs.device_file_ops_deferred());
        assertions.assert("uevent deferred", ctx.devfs.uevent_deferred());
        assertions.assert("sysfs deferred", ctx.devfs.sysfs_deferred());
        assertions.assert(
            "hwrng core current",
            ctx.hwrng_core.current_device().is_some(),
        );
        assertions.assert(
            "block default",
            ctx.block_device_registry.default_entry().is_some(),
        );
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        let ctx = context();
        let Some(mount_ref) = ctx.devfs.mount_ref() else {
            assertions.assert("devfs mount ref", false);
            return;
        };
        let Some(mount) = ctx.vfs_core.mount(mount_ref) else {
            assertions.assert("devfs mount", false);
            return;
        };
        assertions.assert("mount kind", mount.fs_kind() == FileSystemKind::DevFs);
        let Some(superblock) = ctx.vfs_core.superblock(mount.superblock_ref()) else {
            assertions.assert("devfs superblock", false);
            return;
        };
        assertions.assert(
            "superblock kind",
            superblock.fs_kind() == FileSystemKind::DevFs,
        );
        assertions.assert("superblock private", superblock.devfs_private_bound());

        let Some(mount_point_ref) = ctx.devfs.mount_point_ref() else {
            assertions.assert("devfs mount point ref", false);
            return;
        };
        let Some(root_ref) = ctx.devfs.root_dentry_ref() else {
            assertions.assert("devfs root ref", false);
            return;
        };
        assertions.assert("mount root", mount.root_dentry_ref() == root_ref);
        assertions.assert(
            "mount point",
            mount.mount_point_ref() == Some(mount_point_ref),
        );

        let Some(mount_point) = ctx.vfs_core.dentry(mount_point_ref) else {
            assertions.assert("mount point dentry", false);
            return;
        };
        assertions.assert("mount point name", mount_point.name() == b"dev");
        assertions.assert(
            "mount redirects",
            mount_point.mounted_root() == Some(root_ref),
        );

        let Some(root) = ctx.vfs_core.dentry(root_ref) else {
            assertions.assert("devfs root dentry", false);
            return;
        };
        let Some(root_inode) = ctx.vfs_core.inode(root.inode_ref()) else {
            assertions.assert("devfs root inode", false);
            return;
        };
        assertions.assert("root dir", root_inode.kind() == VfsInodeKind::Directory);

        let Some(hwrng_node) = ctx.devfs.hwrng_node() else {
            assertions.assert("hwrng node", false);
            return;
        };
        let Some(block_node) = ctx.devfs.block_node() else {
            assertions.assert("block node", false);
            return;
        };
        assertions.assert("hwrng node name", hwrng_node.name() == b"hwrng");
        assertions.assert("hwrng node kind", hwrng_node.kind() == DevFsNodeKind::HwRng);
        assertions.assert(
            "hwrng binding",
            hwrng_node.hwrng_device_ref() == ctx.hwrng_core.current_device(),
        );

        let Some(default_entry) = ctx.block_device_registry.default_entry() else {
            assertions.assert("default block entry", false);
            return;
        };
        assertions.assert("block node name", block_node.name() == default_entry.name());
        assertions.assert(
            "block node kind",
            block_node.kind() == DevFsNodeKind::BlockDevice,
        );
        assertions.assert(
            "block binding",
            block_node.block_device_ref() == ctx.block_device_registry.default_device(),
        );
        assertions.assert(
            "block devt",
            block_node.devt() == Some(default_entry.devt()),
        );

        assertions.assert(
            "lookup hwrng",
            ctx.vfs_core.lookup_child(mount_point_ref, b"hwrng") == Ok(hwrng_node.dentry_ref()),
        );
        assertions.assert(
            "lookup block",
            ctx.vfs_core
                .lookup_child(mount_point_ref, default_entry.name())
                == Ok(block_node.dentry_ref()),
        );

        let entries = match ctx.vfs_core.read_dir(mount_point_ref) {
            Ok(entries) => entries,
            Err(_) => {
                assertions.assert("list /dev", false);
                return;
            }
        };
        assertions.assert(
            "list hwrng",
            entries.iter().any(|entry| {
                entry.name_eq(b"hwrng")
                    && entry.dentry_ref() == hwrng_node.dentry_ref()
                    && entry.kind() == VfsInodeKind::DeviceNode
            }),
        );
        assertions.assert(
            "list block",
            entries.iter().any(|entry| {
                entry.name_eq(default_entry.name())
                    && entry.dentry_ref() == block_node.dentry_ref()
                    && entry.kind() == VfsInodeKind::DeviceNode
            }),
        );
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}
