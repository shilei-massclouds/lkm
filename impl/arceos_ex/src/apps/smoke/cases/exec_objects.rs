use crate::{
    apps::smoke::SmokeResult,
    context::context,
    objects::{
        exec_transaction, printk,
        user_boot::{USER_BOOT_READ_MAX, USER_INIT_PATH},
        virtio_blk,
    },
};

static mut EXEC_ROLLBACK_READ_BUFFER: [u8; USER_BOOT_READ_MAX] = [0; USER_BOOT_READ_MAX];

pub fn run() -> SmokeResult {
    let ctx = context();
    let mut provider = virtio_blk::live_provider(&ctx.kernel_image);
    let buffer = unsafe { &mut *core::ptr::addr_of_mut!(EXEC_ROLLBACK_READ_BUFFER) };
    buffer.fill(0);
    let Ok(len) = ctx.vfs_core.read_path(
        &ctx.fs_struct,
        &mut ctx.ext2_filesystem,
        &mut ctx.block_device_registry,
        &mut provider,
        USER_INIT_PATH,
        buffer,
    ) else {
        printk::write_str("exec rollback smoke image read failed\n");
        return SmokeResult::Failed;
    };

    if !exec_transaction::smoke_abort_releases_staging(ctx, &buffer[..len]) {
        printk::write_str("exec rollback smoke invariant failed\n");
        return SmokeResult::Failed;
    }
    if !exec_transaction::smoke_entropy_failure_preserves_current(ctx) {
        printk::write_str("exec entropy rollback smoke invariant failed\n");
        return SmokeResult::Failed;
    }
    if !exec_transaction::smoke_consecutive_exec_randoms_differ(ctx) {
        printk::write_str("exec consecutive AT_RANDOM smoke invariant failed\n");
        return SmokeResult::Failed;
    }
    if !super::user_boot::stage_user_elf_images() {
        printk::write_str("exec object smoke could not stage user ELF images\n");
        return SmokeResult::Failed;
    }

    printk::write_str("exec rollback restored free pages and current image\n");
    SmokeResult::Passed
}
