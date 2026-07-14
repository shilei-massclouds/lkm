use crate::{
    context::Context,
    objects::{printk, state::EventResult, user_boot},
};

pub(crate) fn prepare(ctx: &mut Context) -> EventResult {
    printk::write_str("arceos_ex user boot start\n");
    user_boot::prepare_first_user_init(
        &mut ctx.user_boot_payload,
        &mut ctx.elf_object,
        &mut ctx.elf_interpreter_object,
        &mut ctx.user_address_space,
        &mut ctx.user_stack,
        &mut ctx.user_trap_frame,
        &mut ctx.user_child_process,
        &mut ctx.user_init_process,
        &mut ctx.vfs_core,
        &ctx.fs_struct,
        &mut ctx.files_struct,
        &mut ctx.ext2_filesystem,
        &mut ctx.block_device_registry,
        &ctx.kernel_init_task,
        &ctx.payload_exec_sync_boundaries,
        ctx.vm.swapper_vm(),
        &ctx.kernel_image,
        &mut ctx.page_allocator,
        &ctx.page_metadata_map,
        &ctx.kernel_global_allocator,
        &mut ctx.exception_stream,
        &mut ctx.syscall_table,
        &ctx.boot_param,
        &ctx.static_command_line,
        &mut ctx.vmalloc_allocator,
        &mut ctx.page_table_caches,
        &ctx.config,
    )
}

pub(crate) fn enter(ctx: &mut Context) -> ! {
    user_boot::enter_first_user_init(
        &ctx.user_boot_payload,
        &ctx.user_address_space,
        &ctx.user_trap_frame,
        &ctx.user_init_process,
    )
}
