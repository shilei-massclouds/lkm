use crate::{context::context, objects::printk, objects::user_boot};

pub fn run() -> ! {
    printk::write_str("arceos_ex user boot start\n");
    let ctx = context();

    user_boot::run_first_user_init(
        &mut ctx.user_boot_payload,
        &mut ctx.elf_object,
        &mut ctx.user_address_space,
        &mut ctx.user_stack,
        &mut ctx.user_trap_frame,
        &mut ctx.vfs_core,
        &ctx.fs_struct,
        &mut ctx.ext2_filesystem,
        &mut ctx.block_device_registry,
        &ctx.kernel_init_task,
        ctx.vm.swapper_vm(),
        &ctx.kernel_image,
        &mut ctx.page_allocator,
        &ctx.page_metadata_map,
        &ctx.kernel_global_allocator,
        &mut ctx.exception_stream,
    )
}
