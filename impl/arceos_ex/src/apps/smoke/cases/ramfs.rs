use crate::{
    apps::smoke::{
        harness::{SmokeAssertions, SmokeScenario, SmokeSuite},
        SmokeResult,
    },
    context::context,
    objects::{
        state::State,
        vfs::{FileSystemKind, VfsInodeKind},
    },
    phases,
};

const MOUNT_POINT_NAME: &[u8] = b"mnt_ramfs";
const DIR_NAME: &[u8] = b"work";
const FILE_NAME: &[u8] = b"sample";
const FILE_DATA: &[u8] = b"ramfs smoke data\n";

pub fn run() -> SmokeResult {
    let mut suite = SmokeSuite::new();
    suite.scenario(&mut RamFsOperationsScenario::new());
    suite.result()
}

struct RamFsOperationsScenario;

impl RamFsOperationsScenario {
    const fn new() -> Self {
        Self
    }
}

impl SmokeScenario for RamFsOperationsScenario {
    fn name(&self) -> &'static str {
        "ramfs.mount_lookup_insert_remove_list_read_write"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        let ctx = context();
        assertions.assert(
            "process prepare ready",
            phases::interrupt::process_prepare::is_ready(),
        );
        assertions.assert("vfs core ready", ctx.vfs_core.state() == State::Ready);
        assertions.assert("ramfs ready", ctx.ramfs_type.state() == State::Ready);
        assertions.assert("ramfs name", ctx.ramfs_type.name() == b"ramfs");
        assertions.assert("ramfs registered", ctx.vfs_core.ramfs_registered());
        assertions.assert("rootfs mounted", ctx.vfs_core.rootfs_mount_created());
        assertions.assert("rootfs mount count", ctx.vfs_core.mount_count() >= 1);
        assertions.assert(
            "rootfs superblock count",
            ctx.vfs_core.superblock_count() >= 1,
        );
        assertions.assert("rootfs inode count", ctx.vfs_core.inode_count() >= 1);
        assertions.assert("rootfs dentry count", ctx.vfs_core.dentry_count() >= 1);

        let Some(mount_ref) = ctx.vfs_core.initial_root_mount() else {
            assertions.assert("root mount present", false);
            return;
        };
        let Some(root_dentry_ref) = ctx.vfs_core.initial_root_dentry() else {
            assertions.assert("root dentry present", false);
            return;
        };
        let Some(mount) = ctx.vfs_core.mount(mount_ref) else {
            assertions.assert("root mount valid", false);
            return;
        };
        assertions.assert("root mount ref", mount.mount_ref() == mount_ref);
        assertions.assert("root mount ramfs", mount.fs_kind() == FileSystemKind::RamFs);
        assertions.assert(
            "mount root dentry",
            mount.root_dentry_ref() == root_dentry_ref,
        );

        let Some(superblock) = ctx.vfs_core.superblock(mount.superblock_ref()) else {
            assertions.assert("root superblock valid", false);
            return;
        };
        assertions.assert(
            "superblock ref",
            superblock.superblock_ref() == mount.superblock_ref(),
        );
        let Some(root_inode_ref) = superblock.root_inode_ref() else {
            assertions.assert("root inode present", false);
            return;
        };
        assertions.assert(
            "superblock root dentry",
            superblock.root_dentry_ref() == Some(root_dentry_ref),
        );

        let Some(root_dentry) = ctx.vfs_core.dentry(root_dentry_ref) else {
            assertions.assert("root dentry valid", false);
            return;
        };
        assertions.assert(
            "root dentry ref",
            root_dentry.dentry_ref() == root_dentry_ref,
        );
        let Some(root_inode) = ctx.vfs_core.inode(root_inode_ref) else {
            assertions.assert("root inode valid", false);
            return;
        };
        assertions.assert("root name", root_dentry.name() == b"/");
        assertions.assert(
            "root positive",
            root_dentry.positive() && !root_dentry.removed(),
        );
        assertions.assert(
            "root inode match",
            root_dentry.inode_ref() == root_inode_ref,
        );
        assertions.assert("root inode ref", root_inode.inode_ref() == root_inode_ref);
        assertions.assert(
            "root inode dir",
            root_inode.kind() == VfsInodeKind::Directory,
        );
        assertions.assert("root inode size", root_inode.size() == 0);
        assertions.assert("root inode children ready", root_inode.child_count() >= 1);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        let ctx = context();
        let Some(root_dentry) = ctx
            .rootfs
            .real_mount_point_ref()
            .or_else(|| ctx.vfs_core.initial_root_dentry())
        else {
            assertions.assert("root dentry", false);
            return;
        };

        let mount_point = match ctx.vfs_core.create_dir(root_dentry, MOUNT_POINT_NAME) {
            Ok(dir) => dir,
            Err(_) => {
                assertions.assert("create mount point", false);
                return;
            }
        };
        assertions.assert(
            "lookup mount point before mount",
            ctx.vfs_core.lookup_child(root_dentry, MOUNT_POINT_NAME) == Ok(mount_point),
        );

        let ramfs_mount = match ctx.vfs_core.mount_ramfs_at(&ctx.ramfs_type, mount_point) {
            Ok(mount) => mount,
            Err(_) => {
                assertions.assert("mount ramfs at dir", false);
                return;
            }
        };
        let Some(mount) = ctx.vfs_core.mount(ramfs_mount) else {
            assertions.assert("second mount valid", false);
            return;
        };
        assertions.assert(
            "second mount ramfs",
            mount.fs_kind() == FileSystemKind::RamFs,
        );
        assertions.assert(
            "second mount point",
            mount.mount_point_ref() == Some(mount_point),
        );
        let ramfs_root = mount.root_dentry_ref();
        let Some(mount_point_obj) = ctx.vfs_core.dentry(mount_point) else {
            assertions.assert("mount point dentry", false);
            return;
        };
        assertions.assert(
            "mount point redirects",
            mount_point_obj.mounted_root() == Some(ramfs_root),
        );
        assertions.assert("mount count after mount", ctx.vfs_core.mount_count() >= 2);

        let dir = match ctx.vfs_core.create_dir(mount_point, DIR_NAME) {
            Ok(dir) => dir,
            Err(_) => {
                assertions.assert("create dir", false);
                return;
            }
        };
        assertions.assert(
            "lookup dir",
            ctx.vfs_core.lookup_child(mount_point, DIR_NAME) == Ok(dir),
        );
        assertions.assert(
            "lookup dir from mounted root",
            ctx.vfs_core.lookup_child(ramfs_root, DIR_NAME) == Ok(dir),
        );

        let file_dentry = match ctx.vfs_core.create_file(dir, FILE_NAME) {
            Ok(file) => file,
            Err(_) => {
                assertions.assert("create file", false);
                return;
            }
        };
        assertions.assert(
            "lookup file",
            ctx.vfs_core.lookup_child(dir, FILE_NAME) == Ok(file_dentry),
        );
        let Some(file_dentry_obj) = ctx.vfs_core.dentry(file_dentry) else {
            assertions.assert("file dentry valid", false);
            return;
        };
        let file_inode_ref = file_dentry_obj.inode_ref();

        let entries = match ctx.vfs_core.read_dir(dir) {
            Ok(entries) => entries,
            Err(_) => {
                assertions.assert("list dir", false);
                return;
            }
        };
        assertions.assert(
            "dir lists file",
            entries.iter().any(|entry| {
                entry.name_eq(FILE_NAME)
                    && entry.dentry_ref() == file_dentry
                    && entry.inode_ref() == file_inode_ref
                    && entry.kind() == VfsInodeKind::RegularFile
            }),
        );

        let file = match ctx.vfs_core.open_file(file_dentry) {
            Ok(file) => file,
            Err(_) => {
                assertions.assert("open file", false);
                return;
            }
        };
        assertions.assert("file count", ctx.vfs_core.file_count() == 1);
        let Some(file_obj) = ctx.vfs_core.file(file) else {
            assertions.assert("file object valid", false);
            return;
        };
        assertions.assert("file ref", file_obj.file_ref() == file);
        assertions.assert("file dentry", file_obj.dentry_ref() == file_dentry);
        assertions.assert("file position initial", file_obj.position() == 0);
        let written = match ctx.vfs_core.write_file(file, 0, FILE_DATA) {
            Ok(written) => written,
            Err(_) => {
                assertions.assert("write file", false);
                return;
            }
        };
        assertions.assert("write len", written == FILE_DATA.len());
        let Some(file_obj) = ctx.vfs_core.file(file) else {
            assertions.assert("file after write", false);
            return;
        };
        assertions.assert("file write fact", file_obj.write_committed());
        assertions.assert(
            "file write len",
            file_obj.last_write_len() == FILE_DATA.len(),
        );

        let mut read_buf = [0u8; FILE_DATA.len()];
        let read = match ctx.vfs_core.read_file(file, 0, &mut read_buf) {
            Ok(read) => read,
            Err(_) => {
                assertions.assert("read file", false);
                return;
            }
        };
        assertions.assert("read len", read == FILE_DATA.len());
        assertions.assert("read data", read_buf == FILE_DATA);
        let Some(file_obj) = ctx.vfs_core.file(file) else {
            assertions.assert("file after read", false);
            return;
        };
        assertions.assert("file read fact", file_obj.read_returns_written_data());
        assertions.assert("file read len", file_obj.last_read_len() == FILE_DATA.len());

        assertions.assert_ok("remove file", ctx.vfs_core.remove_child(dir, FILE_NAME));
        assertions.assert_fail(
            "lookup removed file",
            ctx.vfs_core.lookup_child(dir, FILE_NAME),
        );

        let entries_after_file_remove = match ctx.vfs_core.read_dir(dir) {
            Ok(entries) => entries,
            Err(_) => {
                assertions.assert("list after remove", false);
                return;
            }
        };
        assertions.assert("dir empty", entries_after_file_remove.is_empty());

        assertions.assert_ok(
            "remove dir",
            ctx.vfs_core.remove_child(mount_point, DIR_NAME),
        );
        assertions.assert_fail(
            "lookup removed dir",
            ctx.vfs_core.lookup_child(mount_point, DIR_NAME),
        );

        assertions.assert("insert count", ctx.vfs_core.insert_count() >= 3);
        assertions.assert("lookup count", ctx.vfs_core.lookup_count() >= 5);
        assertions.assert("remove count", ctx.vfs_core.remove_count() >= 2);
        assertions.assert("readdir count", ctx.vfs_core.readdir_count() >= 2);
        assertions.assert("write count", ctx.vfs_core.write_count() >= 1);
        assertions.assert("read count", ctx.vfs_core.read_count() >= 1);
        assertions.assert("lookup returned", ctx.vfs_core.lookup_returned());
        assertions.assert("readdir listed", ctx.vfs_core.readdir_lists());
        assertions.assert("write committed", ctx.vfs_core.file_write_committed());
        assertions.assert(
            "read returns written",
            ctx.vfs_core.file_read_returns_written_data(),
        );
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}
