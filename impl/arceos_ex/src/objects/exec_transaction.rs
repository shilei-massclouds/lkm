use alloc::vec::Vec;

use super::{
    binary_format_registry::BinaryFormatRegistry,
    config::ExecArgumentLimits,
    elf_object::{ElfError, ElfObject},
    exec_sync_boundaries::ExecSyncBoundaries,
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
    user_boot::{UserAddressSpace, UserTrapFrame},
    user_stack::UserStack,
};

#[cfg(any(app_smoke, app_user_boot))]
use super::user_stack::UserStackAuxv;

#[cfg(app_user_boot)]
use super::binary_format_registry::{BinaryFormatError, ElfPreparationStage};

#[cfg_attr(not(app_user_boot), allow(dead_code))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecOwner {
    Boot,
    Runtime,
}

#[cfg(any(app_smoke, app_user_boot))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RetiredImageRetention {
    None,
    OuterChild,
    BuiltinGrandchild,
}

#[cfg_attr(not(app_user_boot), allow(dead_code))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecError {
    Busy,
    InvalidState,
    InvalidPath,
    ArgumentsTooBig,
    NotFound,
    NoExecutableFormat(Option<ElfError>),
    NoMemory,
    EntropyUnavailable,
}

pub struct ExecArguments {
    filename: [u8; super::files::FILE_PATH_MAX],
    filename_len: usize,
    argv: Vec<Vec<u8>>,
    envp: Vec<Vec<u8>>,
}

#[allow(dead_code)]
impl ExecArguments {
    pub fn from_slices(
        filename: &[u8],
        argv: &[&[u8]],
        envp: &[&[u8]],
        limits: ExecArgumentLimits,
    ) -> Result<Self, ExecError> {
        let mut arguments = Self::empty();
        arguments.bind_slices(filename, argv, envp, limits)?;
        Ok(arguments)
    }

    pub fn for_boot(filename: &[u8], limits: ExecArgumentLimits) -> Result<Self, ExecError> {
        Self::from_slices(filename, &[filename], &[b"HOME=/", b"TERM=linux"], limits)
    }

    pub fn filename(&self) -> &[u8] {
        &self.filename[..self.filename_len]
    }

    pub const fn argc(&self) -> usize {
        self.argv.len()
    }

    pub const fn envc(&self) -> usize {
        self.envp.len()
    }

    pub fn argv_len(&self, index: usize) -> usize {
        self.argv[index].len()
    }

    pub fn envp_len(&self, index: usize) -> usize {
        self.envp[index].len()
    }

    pub fn argv(&self, index: usize) -> &[u8] {
        &self.argv[index]
    }

    pub fn envp(&self, index: usize) -> &[u8] {
        &self.envp[index]
    }

    const fn empty() -> Self {
        Self {
            filename: [0; super::files::FILE_PATH_MAX],
            filename_len: 0,
            argv: Vec::new(),
            envp: Vec::new(),
        }
    }

    fn bind_slices(
        &mut self,
        filename: &[u8],
        argv: &[&[u8]],
        envp: &[&[u8]],
        limits: ExecArgumentLimits,
    ) -> Result<(), ExecError> {
        if filename.is_empty() || filename[0] != b'/' {
            return Err(ExecError::InvalidPath);
        }
        if argv.is_empty() || filename.len() > super::files::FILE_PATH_MAX {
            return Err(ExecError::ArgumentsTooBig);
        }
        validate_argument_slices(filename, argv, envp, limits)?;
        let copied_argv = copy_argument_vector(argv)?;
        let copied_envp = copy_argument_vector(envp)?;
        *self = Self::empty();
        self.filename[..filename.len()].copy_from_slice(filename);
        self.filename_len = filename.len();
        self.argv = copied_argv;
        self.envp = copied_envp;
        Ok(())
    }
}

fn validate_argument_slices(
    filename: &[u8],
    argv: &[&[u8]],
    envp: &[&[u8]],
    limits: ExecArgumentLimits,
) -> Result<(), ExecError> {
    let mut string_bytes = filename
        .len()
        .checked_add(1)
        .ok_or(ExecError::ArgumentsTooBig)?;
    for value in argv.iter().chain(envp.iter()) {
        let bytes_with_nul = value
            .len()
            .checked_add(1)
            .ok_or(ExecError::ArgumentsTooBig)?;
        if bytes_with_nul > limits.max_arg_strlen() {
            return Err(ExecError::ArgumentsTooBig);
        }
        string_bytes = string_bytes
            .checked_add(bytes_with_nul)
            .ok_or(ExecError::ArgumentsTooBig)?;
    }
    if !limits.accepts(argv.len(), envp.len(), string_bytes) {
        return Err(ExecError::ArgumentsTooBig);
    }
    Ok(())
}

fn copy_argument_vector(source: &[&[u8]]) -> Result<Vec<Vec<u8>>, ExecError> {
    let mut copied = Vec::new();
    copied
        .try_reserve_exact(source.len())
        .map_err(|_| ExecError::NoMemory)?;
    for value in source {
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(value.len())
            .map_err(|_| ExecError::NoMemory)?;
        bytes.extend_from_slice(value);
        copied.push(bytes);
    }
    Ok(copied)
}

#[allow(dead_code)]
#[derive(Clone, Copy)]
pub struct ExecSuccess {
    pub image_contains_stdin_fixture: bool,
    pub retired_pages_released: usize,
}

#[cfg(any(app_smoke, app_user_boot))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ExecEntropy {
    at_random: [u8; super::user_stack::USER_STACK_RANDOM_BYTES],
    stack_aslr: [u8; super::user_stack::USER_STACK_ASLR_BYTES],
}

#[cfg_attr(not(app_user_boot), allow(dead_code))]
pub struct ExecTransaction {
    lifecycle: Lifecycle,
    active: bool,
    owner: ExecOwner,
    arguments: ExecArguments,
    point_of_no_return: bool,
    commit_count: usize,
    abort_count: usize,
    last_error: Option<ExecError>,
    last_retired_pages_released: usize,
    pub(crate) staging_elf: ElfObject,
    pub(crate) staging_interpreter: ElfObject,
    pub(crate) staging_address_space: UserAddressSpace,
    pub(crate) staging_stack: UserStack,
    pub(crate) staging_trap_frame: UserTrapFrame,
    pub(crate) retired_address_space: UserAddressSpace,
    pub(crate) retired_stack: UserStack,
}

#[allow(dead_code)]
impl ExecTransaction {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            active: false,
            owner: ExecOwner::Boot,
            arguments: ExecArguments::empty(),
            point_of_no_return: false,
            commit_count: 0,
            abort_count: 0,
            last_error: None,
            last_retired_pages_released: 0,
            staging_elf: ElfObject::new(),
            staging_interpreter: ElfObject::new(),
            staging_address_space: UserAddressSpace::new(),
            staging_stack: UserStack::new(),
            staging_trap_frame: UserTrapFrame::new(),
            retired_address_space: UserAddressSpace::new(),
            retired_stack: UserStack::new(),
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn active(&self) -> bool {
        self.active
    }

    pub const fn point_of_no_return(&self) -> bool {
        self.point_of_no_return
    }

    pub const fn commit_count(&self) -> usize {
        self.commit_count
    }

    pub const fn abort_count(&self) -> usize {
        self.abort_count
    }

    pub const fn last_error(&self) -> Option<ExecError> {
        self.last_error
    }

    pub const fn last_retired_pages_released(&self) -> usize {
        self.last_retired_pages_released
    }

    pub(crate) fn active_filename(&self) -> Option<&[u8]> {
        if self.active {
            Some(self.arguments.filename())
        } else {
            None
        }
    }

    pub fn setup(
        &mut self,
        registry: &BinaryFormatRegistry,
        sync: &ExecSyncBoundaries,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || registry.state() != State::Ready
            || !registry.only_elf_handler()
            || sync.state() != State::Ready
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }

    fn begin(&mut self, owner: ExecOwner, arguments: ExecArguments) -> Result<(), ExecError> {
        if self.lifecycle.state() != State::Ready {
            return Err(ExecError::InvalidState);
        }
        if self.active {
            return Err(ExecError::Busy);
        }
        self.active = true;
        self.owner = owner;
        self.arguments = arguments;
        self.point_of_no_return = false;
        self.last_error = None;
        self.last_retired_pages_released = 0;
        Ok(())
    }

    fn begin_slices(
        &mut self,
        owner: ExecOwner,
        filename: &[u8],
        argv: &[&[u8]],
        envp: &[&[u8]],
        limits: ExecArgumentLimits,
    ) -> Result<(), ExecError> {
        if self.lifecycle.state() != State::Ready {
            return Err(ExecError::InvalidState);
        }
        if self.active {
            return Err(ExecError::Busy);
        }
        self.arguments.bind_slices(filename, argv, envp, limits)?;
        self.active = true;
        self.owner = owner;
        self.point_of_no_return = false;
        self.last_error = None;
        self.last_retired_pages_released = 0;
        Ok(())
    }

    fn mark_point_of_no_return(&mut self) -> Result<(), ExecError> {
        if !self.active || self.point_of_no_return {
            return Err(ExecError::InvalidState);
        }
        self.point_of_no_return = true;
        Ok(())
    }

    fn finish_commit(&mut self, retired_pages_released: usize) {
        self.active = false;
        self.point_of_no_return = false;
        self.arguments = ExecArguments::empty();
        self.commit_count += 1;
        self.last_retired_pages_released = retired_pages_released;
    }

    fn abort(
        &mut self,
        error: ExecError,
        page_allocator: &mut super::mm_core::PageAllocator,
        page_metadata_map: &super::mm_core::PageMetadataMap,
    ) {
        if self.point_of_no_return {
            exec_terminal("exec abort after point-of-no-return\n");
        }
        self.staging_address_space
            .discard_staging_after_exec_failure(page_allocator, page_metadata_map);
        self.staging_stack
            .release_exec_backing(page_allocator, page_metadata_map);
        self.staging_elf = ElfObject::new();
        self.staging_interpreter = ElfObject::new();
        self.staging_trap_frame = UserTrapFrame::new();
        self.active = false;
        self.arguments = ExecArguments::empty();
        self.last_error = Some(error);
        self.abort_count += 1;
    }
}

#[cfg(app_user_boot)]
static mut EXEC_MAIN_READ_BUFFER: [u8; super::user_boot::USER_BOOT_READ_MAX] =
    [0; super::user_boot::USER_BOOT_READ_MAX];
#[cfg(app_user_boot)]
static mut EXEC_INTERPRETER_READ_BUFFER: [u8; super::user_boot::USER_BOOT_READ_MAX] =
    [0; super::user_boot::USER_BOOT_READ_MAX];

#[cfg(app_user_boot)]
#[allow(dead_code)]
pub fn execute(
    ctx: &mut crate::context::Context,
    arguments: ExecArguments,
    owner: ExecOwner,
    runtime_frame: Option<&mut super::trap_type::TrapFrame>,
) -> Result<ExecSuccess, ExecError> {
    ctx.exec_transaction.begin(owner, arguments)?;
    let result = prepare_and_commit(ctx, runtime_frame);
    if let Err(error) = result {
        if ctx.exec_transaction.point_of_no_return() {
            exec_terminal("exec failure after point-of-no-return\n");
        }
        ctx.exec_transaction
            .abort(error, &mut ctx.page_allocator, &ctx.page_metadata_map);
    }
    result
}

#[cfg(app_user_boot)]
pub fn execute_slices(
    ctx: &mut crate::context::Context,
    filename: &[u8],
    argv: &[&[u8]],
    envp: &[&[u8]],
    owner: ExecOwner,
    runtime_frame: Option<&mut super::trap_type::TrapFrame>,
) -> Result<ExecSuccess, ExecError> {
    let limits = ctx.config.exec_argument_limits();
    ctx.exec_transaction
        .begin_slices(owner, filename, argv, envp, limits)?;
    let result = prepare_and_commit(ctx, runtime_frame);
    if let Err(error) = result {
        if ctx.exec_transaction.point_of_no_return() {
            exec_terminal("exec failure after point-of-no-return\n");
        }
        ctx.exec_transaction
            .abort(error, &mut ctx.page_allocator, &ctx.page_metadata_map);
    }
    result
}

#[cfg(app_smoke)]
pub fn smoke_abort_releases_staging(ctx: &mut crate::context::Context, image: &[u8]) -> bool {
    super::printk::write_str("exec rollback smoke: begin\n");
    let free_pages_before = ctx.page_allocator.buddy_total_free_pages();
    let current_satp_before = ctx.user_address_space.satp_token();
    let current_stack_state_before = ctx.user_stack.state();
    let abort_count_before = ctx.exec_transaction.abort_count();
    if ctx
        .exec_transaction
        .begin_slices(
            ExecOwner::Boot,
            b"/sbin/init",
            &[b"/sbin/init"],
            &[b"HOME=/"],
            ctx.config.exec_argument_limits(),
        )
        .is_err()
    {
        return false;
    }

    let prepared = (|| -> Result<(), ExecError> {
        ctx.binary_format_registry
            .prepare_main(image, &mut ctx.exec_transaction.staging_elf)
            .map_err(|_| ExecError::NoExecutableFormat(None))?;
        super::printk::write_str("exec rollback smoke: elf ready\n");
        ctx.exec_transaction
            .staging_address_space
            .preset(
                ctx.vm.swapper_vm(),
                &ctx.page_allocator,
                &ctx.kernel_global_allocator,
                &ctx.kernel_init_task,
            )
            .map_err(|_| ExecError::NoMemory)?;
        super::printk::write_str("exec rollback smoke: address preset\n");
        ctx.exec_transaction
            .staging_stack
            .setup_with_envp(
                &ctx.exec_transaction.staging_address_space,
                &ctx.exec_transaction.staging_elf,
                None,
                &[b"/sbin/init"],
                &[b"HOME=/"],
                ctx.config.user_stack(),
                b"/sbin/init",
                &[0x5a; super::user_stack::USER_STACK_RANDOM_BYTES],
                &[0; super::user_stack::USER_STACK_ASLR_BYTES],
                UserStackAuxv::root(ctx.cpu_capabilities.elf_hwcap()),
                &mut ctx.page_allocator,
                &ctx.page_metadata_map,
            )
            .map_err(|_| ExecError::NoMemory)?;
        super::printk::write_str("exec rollback smoke: stack ready\n");
        ctx.exec_transaction
            .staging_address_space
            .setup(
                &ctx.exec_transaction.staging_elf,
                None,
                &ctx.exec_transaction.staging_stack,
                image,
                None,
                &mut ctx.page_allocator,
                &ctx.page_metadata_map,
            )
            .map_err(|_| ExecError::NoMemory)?;
        super::printk::write_str("exec rollback smoke: address ready\n");
        Ok(())
    })();

    if let Err(error) = prepared {
        ctx.exec_transaction
            .abort(error, &mut ctx.page_allocator, &ctx.page_metadata_map);
        return false;
    }
    let allocated_pages_observed = ctx.page_allocator.buddy_total_free_pages() < free_pages_before;
    ctx.exec_transaction.abort(
        ExecError::InvalidState,
        &mut ctx.page_allocator,
        &ctx.page_metadata_map,
    );
    super::printk::write_str("exec rollback smoke: aborted\n");

    allocated_pages_observed
        && ctx.page_allocator.buddy_total_free_pages() == free_pages_before
        && ctx.user_address_space.satp_token() == current_satp_before
        && ctx.user_stack.state() == current_stack_state_before
        && ctx.exec_transaction.state() == State::Ready
        && !ctx.exec_transaction.active()
        && !ctx.exec_transaction.point_of_no_return()
        && ctx.exec_transaction.abort_count() == abort_count_before + 1
        && ctx.exec_transaction.last_error() == Some(ExecError::InvalidState)
}

#[cfg(app_smoke)]
pub fn smoke_entropy_failure_preserves_current(ctx: &mut crate::context::Context) -> bool {
    let free_pages_before = ctx.page_allocator.buddy_total_free_pages();
    let satp_before = ctx.user_address_space.satp_token();
    let stack_pages_before = ctx.user_stack.backing_page_count();
    let abort_count_before = ctx.exec_transaction.abort_count();
    if ctx
        .exec_transaction
        .begin_slices(
            ExecOwner::Runtime,
            b"/sbin/init",
            &[b"different-argv0"],
            &[],
            ctx.config.exec_argument_limits(),
        )
        .is_err()
        || ctx
            .exec_transaction
            .staging_address_space
            .preset(
                ctx.vm.swapper_vm(),
                &ctx.page_allocator,
                &ctx.kernel_global_allocator,
                &ctx.kernel_init_task,
            )
            .is_err()
    {
        return false;
    }
    let mut unavailable_runtime = super::virtio_rng::VirtioRngRuntime::new();
    let mut unavailable_core = super::hwrng::HwRngCore::new();
    let unavailable = acquire_exec_entropy(&mut unavailable_runtime, &mut unavailable_core);
    let short = validate_exec_entropy_len(super::user_stack::USER_STACK_ENTROPY_BYTES - 1);
    let entropy_failed = unavailable == Err(ExecError::EntropyUnavailable)
        && short == Err(ExecError::EntropyUnavailable)
        && !ctx.exec_transaction.point_of_no_return();
    ctx.exec_transaction.abort(
        ExecError::EntropyUnavailable,
        &mut ctx.page_allocator,
        &ctx.page_metadata_map,
    );
    entropy_failed
        && ctx.page_allocator.buddy_total_free_pages() == free_pages_before
        && ctx.user_address_space.satp_token() == satp_before
        && ctx.user_stack.backing_page_count() == stack_pages_before
        && !ctx.exec_transaction.active()
        && !ctx.exec_transaction.point_of_no_return()
        && ctx.exec_transaction.abort_count() == abort_count_before + 1
        && ctx.exec_transaction.last_error() == Some(ExecError::EntropyUnavailable)
}

#[cfg(app_smoke)]
pub fn smoke_consecutive_exec_randoms_differ(ctx: &mut crate::context::Context) -> bool {
    let Ok(first) = acquire_exec_entropy(&mut ctx.virtio_rng_runtime, &mut ctx.hwrng_core) else {
        return false;
    };
    let Ok(second) = acquire_exec_entropy(&mut ctx.virtio_rng_runtime, &mut ctx.hwrng_core) else {
        return false;
    };
    first.at_random.iter().any(|byte| *byte != 0)
        && first.stack_aslr.iter().any(|byte| *byte != 0)
        && second.at_random.iter().any(|byte| *byte != 0)
        && second.stack_aslr.iter().any(|byte| *byte != 0)
        && first != second
}

#[cfg(app_smoke)]
pub fn smoke_builtin_grandchild_precommit_failure_is_atomic(
    ctx: &mut crate::context::Context,
) -> bool {
    if !ctx.user_task_set.builtin_grandchild_active()
        || !ctx
            .user_task_set
            .builtin_grandchild_first_exec_retention_required()
    {
        return false;
    }
    let free_pages_before = ctx.page_allocator.buddy_total_free_pages();
    let current_satp_before = ctx.user_address_space.satp_token();
    let current_stack_top_before = ctx.user_stack.top();
    let open_fds_before = ctx.files_struct.fd_table_open_count();
    let outer_snapshot_before = ctx.user_task_set.parent_address_space_snapshot_saved();
    let abort_count_before = ctx.exec_transaction.abort_count();
    if ctx
        .exec_transaction
        .begin_slices(
            ExecOwner::Runtime,
            b"/bin/sh",
            &[b"/bin/sh", b"-c", b"uname01"],
            &[b"HOME=/"],
            ctx.config.exec_argument_limits(),
        )
        .is_err()
        || ctx
            .exec_transaction
            .staging_address_space
            .preset(
                ctx.vm.swapper_vm(),
                &ctx.page_allocator,
                &ctx.kernel_global_allocator,
                &ctx.kernel_init_task,
            )
            .is_err()
    {
        return false;
    }
    ctx.exec_transaction.abort(
        ExecError::InvalidState,
        &mut ctx.page_allocator,
        &ctx.page_metadata_map,
    );
    ctx.page_allocator.buddy_total_free_pages() == free_pages_before
        && ctx.user_address_space.satp_token() == current_satp_before
        && ctx.user_stack.top() == current_stack_top_before
        && ctx.files_struct.fd_table_open_count() == open_fds_before
        && ctx.user_task_set.parent_address_space_snapshot_saved() == outer_snapshot_before
        && ctx
            .user_task_set
            .builtin_grandchild_first_exec_retention_required()
        && ctx.user_task_set.builtin_grandchild_exec_commit_count() == 0
        && !ctx
            .user_task_set
            .builtin_grandchild_parent_exec_snapshot_ever_saved()
        && ctx.exec_transaction.abort_count() == abort_count_before + 1
        && ctx.exec_transaction.last_error() == Some(ExecError::InvalidState)
}

#[cfg(app_smoke)]
pub fn smoke_commit_builtin_grandchild_exec_image(
    ctx: &mut crate::context::Context,
    main_image: &[u8],
    interpreter_image: Option<&[u8]>,
) -> Option<ExecSuccess> {
    if !ctx.user_task_set.builtin_grandchild_active() {
        return None;
    }
    ctx.exec_transaction
        .begin_slices(
            ExecOwner::Runtime,
            b"/bin/sh",
            &[b"/bin/sh", b"-c", b"uname01"],
            &[b"HOME=/", b"TERM=linux"],
            ctx.config.exec_argument_limits(),
        )
        .ok()?;

    let prepared = (|| -> Result<(), ExecError> {
        ctx.binary_format_registry
            .prepare_main(main_image, &mut ctx.exec_transaction.staging_elf)
            .map_err(|_| ExecError::NoExecutableFormat(None))?;
        let interpreter_ref = if ctx
            .exec_transaction
            .staging_elf
            .interpreter_path()
            .is_some()
        {
            let image = interpreter_image.ok_or(ExecError::NotFound)?;
            ctx.binary_format_registry
                .prepare_interpreter(image, &mut ctx.exec_transaction.staging_interpreter)
                .map_err(|_| ExecError::NoExecutableFormat(None))?;
            ctx.exec_transaction
                .staging_elf
                .bind_runtime_interpreter(&ctx.exec_transaction.staging_interpreter)
                .map_err(|_| ExecError::InvalidState)?;
            Some(&ctx.exec_transaction.staging_interpreter)
        } else {
            None
        };
        ctx.exec_transaction
            .staging_address_space
            .preset(
                ctx.vm.swapper_vm(),
                &ctx.page_allocator,
                &ctx.kernel_global_allocator,
                &ctx.kernel_init_task,
            )
            .map_err(|_| ExecError::NoMemory)?;
        ctx.exec_transaction
            .staging_stack
            .setup_with_envp(
                &ctx.exec_transaction.staging_address_space,
                &ctx.exec_transaction.staging_elf,
                interpreter_ref,
                &[b"/bin/sh", b"-c", b"uname01"],
                &[b"HOME=/", b"TERM=linux"],
                ctx.config.user_stack(),
                b"/bin/sh",
                &[0x5a; super::user_stack::USER_STACK_RANDOM_BYTES],
                &[0; super::user_stack::USER_STACK_ASLR_BYTES],
                UserStackAuxv::new(
                    ctx.cpu_capabilities.elf_hwcap(),
                    ctx.kernel_init_user_state.uid(),
                    ctx.kernel_init_user_state.euid(),
                    ctx.kernel_init_user_state.gid(),
                    ctx.kernel_init_user_state.egid(),
                ),
                &mut ctx.page_allocator,
                &ctx.page_metadata_map,
            )
            .map_err(|_| ExecError::NoMemory)?;
        ctx.exec_transaction
            .staging_address_space
            .setup(
                &ctx.exec_transaction.staging_elf,
                interpreter_ref,
                &ctx.exec_transaction.staging_stack,
                main_image,
                interpreter_image,
                &mut ctx.page_allocator,
                &ctx.page_metadata_map,
            )
            .map_err(|_| ExecError::NoMemory)?;
        ctx.exec_transaction
            .staging_trap_frame
            .setup(
                &ctx.exec_transaction.staging_address_space,
                &ctx.exec_transaction.staging_elf,
                &ctx.exec_transaction.staging_stack,
            )
            .map_err(|_| ExecError::InvalidState)?;
        ctx.exec_transaction
            .staging_elf
            .enable(
                &ctx.exec_transaction.staging_address_space,
                &ctx.exec_transaction.staging_stack,
                &ctx.exec_transaction.staging_trap_frame,
            )
            .map_err(|_| ExecError::InvalidState)?;
        ctx.exec_transaction
            .staging_address_space
            .enable(
                &ctx.exec_transaction.staging_trap_frame,
                &ctx.exec_transaction.staging_stack,
                ctx.vm.swapper_vm(),
                &ctx.kernel_image,
                &mut ctx.page_allocator,
                &ctx.page_metadata_map,
            )
            .map_err(|_| ExecError::NoMemory)?;
        ctx.files_struct
            .precheck_close_on_exec()
            .map_err(|_| ExecError::InvalidState)
    })();
    if let Err(error) = prepared {
        ctx.exec_transaction
            .abort(error, &mut ctx.page_allocator, &ctx.page_metadata_map);
        return None;
    }

    let retention = match classify_retired_image_retention(ctx, ExecOwner::Runtime) {
        Ok(retention) => retention,
        Err(_) => {
            ctx.exec_transaction.abort(
                ExecError::InvalidState,
                &mut ctx.page_allocator,
                &ctx.page_metadata_map,
            );
            return None;
        }
    };
    let builtin_subsequent_exec = retention == RetiredImageRetention::None
        && ctx
            .user_task_set
            .builtin_grandchild_exec_parent_snapshot_live();
    ctx.exec_transaction.mark_point_of_no_return().ok()?;
    if !commit_runtime_flow_handoff(ctx) {
        return None;
    }
    let mut directly_released = 0;
    if builtin_subsequent_exec {
        directly_released += ctx
            .user_address_space
            .release_retired_exec_image(&mut ctx.page_allocator, &ctx.page_metadata_map);
        directly_released += ctx
            .user_stack
            .release_exec_backing(&mut ctx.page_allocator, &ctx.page_metadata_map);
    } else {
        unsafe {
            core::ptr::copy_nonoverlapping(
                &ctx.user_address_space,
                &mut ctx.exec_transaction.retired_address_space,
                1,
            );
        }
    }
    unsafe {
        core::ptr::copy_nonoverlapping(
            &ctx.exec_transaction.staging_address_space,
            &mut ctx.user_address_space,
            1,
        );
    }
    ctx.exec_transaction
        .staging_address_space
        .reset_staging_after_exec_commit();
    let _ = ctx.files_struct.close_on_exec().ok()?;
    if !builtin_subsequent_exec {
        core::mem::swap(&mut ctx.user_stack, &mut ctx.exec_transaction.retired_stack);
    }
    core::mem::swap(&mut ctx.user_stack, &mut ctx.exec_transaction.staging_stack);
    ctx.exec_transaction
        .staging_stack
        .reset_staging_after_exec_commit();
    ctx.exec_transaction.staging_elf = ElfObject::new();
    ctx.exec_transaction.staging_interpreter = ElfObject::new();
    ctx.exec_transaction.staging_trap_frame = UserTrapFrame::new();

    match retention {
        RetiredImageRetention::BuiltinGrandchild => {
            if !ctx.user_task_set.retain_builtin_grandchild_exec_objects(
                &ctx.exec_transaction.retired_address_space,
                &mut ctx.exec_transaction.retired_stack,
            ) {
                return None;
            }
        }
        RetiredImageRetention::None => {}
        RetiredImageRetention::OuterChild => return None,
    }
    let released = if builtin_subsequent_exec {
        if !ctx
            .user_task_set
            .mark_builtin_grandchild_subsequent_exec_committed()
        {
            return None;
        }
        directly_released
    } else {
        0
    };
    ctx.exec_transaction.finish_commit(released);
    Some(ExecSuccess {
        image_contains_stdin_fixture: super::elf_object::contains_bytes(
            main_image,
            b"user-smoke: begin",
        ),
        retired_pages_released: released,
    })
}

#[cfg(app_user_boot)]
fn prepare_and_commit(
    ctx: &mut crate::context::Context,
    runtime_frame: Option<&mut super::trap_type::TrapFrame>,
) -> Result<ExecSuccess, ExecError> {
    use super::exception_type as observation;
    use crate::checkpoint::Checkpoint;

    let owner = ctx.exec_transaction.owner;
    let image = read_main_exec_image(ctx).map_err(|_| {
        observe_failure(
            owner,
            observation::EXECVE_FAIL_STAGE_PATH_READ,
            observation::EXECVE_FAIL_REASON_VFS_READ,
            0,
            ctx,
        );
        ExecError::NotFound
    })?;
    if owner == ExecOwner::Runtime {
        observation::record_execve_main_image_read(image.len());
    }

    if let Err(error) = ctx
        .binary_format_registry
        .prepare_main(image, &mut ctx.exec_transaction.staging_elf)
    {
        let (exec_error, stage, detail) = format_error(error, true);
        observe_failure(
            owner,
            stage,
            observation::EXECVE_FAIL_REASON_UNSUPPORTED_ELF,
            detail,
            ctx,
        );
        return Err(exec_error);
    }
    if owner == ExecOwner::Runtime {
        observation::record_execve_main_elf(&ctx.exec_transaction.staging_elf);
        crate::checkpoint::dispatch(Checkpoint::UserExecMainElfReady, ctx);
    } else {
        crate::checkpoint::dispatch(Checkpoint::UserBootMainElfReady, ctx);
    }

    let mut interpreter_path = [0u8; super::files::FILE_PATH_MAX];
    let interpreter_path_len = ctx
        .exec_transaction
        .staging_elf
        .interpreter_path()
        .map(|path| {
            let len = path.len();
            interpreter_path[..len].copy_from_slice(path);
            len
        });
    let interpreter_image = if let Some(path_len) = interpreter_path_len {
        let path = &interpreter_path[..path_len];
        if owner == ExecOwner::Runtime {
            observation::record_execve_interpreter_path(path);
        }
        let image = read_exec_image(ctx, path, true).map_err(|_| {
            observe_failure(
                owner,
                observation::EXECVE_FAIL_STAGE_INTERPRETER_READ,
                observation::EXECVE_FAIL_REASON_INTERPRETER_READ,
                0,
                ctx,
            );
            ExecError::NotFound
        })?;
        if owner == ExecOwner::Runtime {
            observation::record_execve_interpreter_image_read(image.len());
        }
        if let Err(error) = ctx
            .binary_format_registry
            .prepare_interpreter(image, &mut ctx.exec_transaction.staging_interpreter)
        {
            let (exec_error, stage, detail) = format_error(error, false);
            observe_failure(
                owner,
                stage,
                observation::EXECVE_FAIL_REASON_UNSUPPORTED_ELF,
                detail,
                ctx,
            );
            return Err(exec_error);
        }
        if let Err(error) = ctx
            .exec_transaction
            .staging_elf
            .bind_runtime_interpreter(&ctx.exec_transaction.staging_interpreter)
        {
            observe_failure(
                owner,
                observation::EXECVE_FAIL_STAGE_INTERPRETER_BIND,
                observation::EXECVE_FAIL_REASON_INVALID_STATE,
                error.index(),
                ctx,
            );
            return Err(elf_exec_error(error));
        }
        if owner == ExecOwner::Runtime {
            observation::record_execve_interpreter(&ctx.exec_transaction.staging_interpreter);
            crate::checkpoint::dispatch(Checkpoint::UserExecInterpreterReady, ctx);
        } else {
            crate::checkpoint::dispatch(Checkpoint::UserBootInterpreterReady, ctx);
        }
        Some(image)
    } else {
        None
    };

    if ctx
        .exec_transaction
        .staging_address_space
        .preset(
            ctx.vm.swapper_vm(),
            &ctx.page_allocator,
            &ctx.kernel_global_allocator,
            &ctx.kernel_init_task,
        )
        .is_err()
    {
        observe_failure(
            owner,
            observation::EXECVE_FAIL_STAGE_ADDRESS_SPACE_PRESET,
            observation::EXECVE_FAIL_REASON_ADDRESS_SPACE,
            0,
            ctx,
        );
        return Err(ExecError::InvalidState);
    }

    let stack_result = {
        let entropy = acquire_exec_entropy(&mut ctx.virtio_rng_runtime, &mut ctx.hwrng_core)?;
        let auxv = if owner == ExecOwner::Boot {
            UserStackAuxv::root(ctx.cpu_capabilities.elf_hwcap())
        } else {
            UserStackAuxv::new(
                ctx.cpu_capabilities.elf_hwcap(),
                ctx.kernel_init_user_state.uid(),
                ctx.kernel_init_user_state.euid(),
                ctx.kernel_init_user_state.gid(),
                ctx.kernel_init_user_state.egid(),
            )
        };
        let transaction = &mut ctx.exec_transaction;
        let argc = transaction.arguments.argc();
        let envc = transaction.arguments.envc();
        let mut argv = Vec::new();
        let mut envp = Vec::new();
        if argv.try_reserve_exact(argc).is_err() || envp.try_reserve_exact(envc).is_err() {
            return Err(ExecError::NoMemory);
        }
        let mut index = 0usize;
        while index < argc {
            argv.push(transaction.arguments.argv(index));
            index += 1;
        }
        index = 0;
        while index < envc {
            envp.push(transaction.arguments.envp(index));
            index += 1;
        }
        let interpreter_ref = interpreter_image.map(|_| &transaction.staging_interpreter);
        transaction.staging_stack.setup_with_envp(
            &transaction.staging_address_space,
            &transaction.staging_elf,
            interpreter_ref,
            &argv[..argc],
            &envp[..envc],
            ctx.config.user_stack(),
            transaction.arguments.filename(),
            &entropy.at_random,
            &entropy.stack_aslr,
            auxv,
            &mut ctx.page_allocator,
            &ctx.page_metadata_map,
        )
    };
    if stack_result.is_err() {
        observe_failure(
            owner,
            observation::EXECVE_FAIL_STAGE_STACK_SETUP,
            observation::EXECVE_FAIL_REASON_STACK,
            0,
            ctx,
        );
        return Err(ExecError::NoMemory);
    }
    if owner == ExecOwner::Runtime {
        observation::record_execve_stack(&ctx.exec_transaction.staging_stack);
    }
    if owner == ExecOwner::Boot {
        crate::checkpoint::dispatch(Checkpoint::UserBootAddressSpaceSetupStart, ctx);
    }
    let setup_result = {
        let transaction = &mut ctx.exec_transaction;
        let interpreter_ref = interpreter_image.map(|_| &transaction.staging_interpreter);
        transaction.staging_address_space.setup(
            &transaction.staging_elf,
            interpreter_ref,
            &transaction.staging_stack,
            image,
            interpreter_image,
            &mut ctx.page_allocator,
            &ctx.page_metadata_map,
        )
    };
    if let Err(error) = setup_result {
        observe_failure(
            owner,
            observation::EXECVE_FAIL_STAGE_ADDRESS_SPACE_SETUP,
            observation::EXECVE_FAIL_REASON_ADDRESS_SPACE,
            error.index(),
            ctx,
        );
        return Err(elf_exec_error(error));
    }
    if owner == ExecOwner::Runtime {
        observation::record_execve_address_space(
            observation::EXECVE_OBS_STAGE_ADDRESS_SPACE_READY,
            &ctx.exec_transaction.staging_address_space,
        );
        crate::checkpoint::dispatch(Checkpoint::UserExecAddressSpaceReady, ctx);
    } else {
        crate::checkpoint::dispatch(Checkpoint::UserAddressSpaceReady, ctx);
    }

    if ctx
        .exec_transaction
        .staging_trap_frame
        .setup(
            &ctx.exec_transaction.staging_address_space,
            &ctx.exec_transaction.staging_elf,
            &ctx.exec_transaction.staging_stack,
        )
        .is_err()
    {
        observe_failure(
            owner,
            observation::EXECVE_FAIL_STAGE_TRAP_FRAME_SETUP,
            observation::EXECVE_FAIL_REASON_TRAP_FRAME,
            0,
            ctx,
        );
        return Err(ExecError::InvalidState);
    }
    if ctx
        .exec_transaction
        .staging_elf
        .enable(
            &ctx.exec_transaction.staging_address_space,
            &ctx.exec_transaction.staging_stack,
            &ctx.exec_transaction.staging_trap_frame,
        )
        .is_err()
    {
        observe_failure(
            owner,
            observation::EXECVE_FAIL_STAGE_ELF_ENABLE,
            observation::EXECVE_FAIL_REASON_INVALID_STATE,
            0,
            ctx,
        );
        return Err(ExecError::InvalidState);
    }
    if let Err(error) = ctx.exec_transaction.staging_address_space.enable(
        &ctx.exec_transaction.staging_trap_frame,
        &ctx.exec_transaction.staging_stack,
        ctx.vm.swapper_vm(),
        &ctx.kernel_image,
        &mut ctx.page_allocator,
        &ctx.page_metadata_map,
    ) {
        observe_failure(
            owner,
            observation::EXECVE_FAIL_STAGE_ADDRESS_SPACE_ENABLE,
            observation::EXECVE_FAIL_REASON_ADDRESS_SPACE,
            error.index(),
            ctx,
        );
        return Err(elf_exec_error(error));
    }
    if owner == ExecOwner::Runtime && ctx.files_struct.precheck_close_on_exec().is_err() {
        observe_failure(
            owner,
            observation::EXECVE_FAIL_STAGE_CLOSE_ON_EXEC,
            observation::EXECVE_FAIL_REASON_CLOSE_ON_EXEC,
            0,
            ctx,
        );
        return Err(ExecError::InvalidState);
    }

    commit_prepared(ctx, owner, runtime_frame, image)
}

#[cfg(any(app_smoke, app_user_boot))]
fn commit_runtime_flow_handoff(ctx: &mut crate::context::Context) -> bool {
    if ctx.user_task_set.active_task_ref().is_valid() {
        ctx.user_task_set.commit_active_exec_flow_handoff()
    } else if ctx.user_app_flow.state() == State::Online {
        ctx.user_app_flow
            .commit_runtime_exec_handoff(&mut ctx.kernel_init_task)
            .is_ok()
    } else {
        true
    }
}

#[cfg(any(app_smoke, app_user_boot))]
fn acquire_exec_entropy(
    runtime: &mut super::virtio_rng::VirtioRngRuntime,
    hwrng_core: &mut super::hwrng::HwRngCore,
) -> Result<ExecEntropy, ExecError> {
    let mut entropy = [0u8; super::user_stack::USER_STACK_ENTROPY_BYTES];
    let len = runtime
        .read_current_hwrng(hwrng_core, &mut entropy, false)
        .map_err(|_| ExecError::EntropyUnavailable)?;
    validate_exec_entropy_len(len)?;
    let mut at_random = [0; super::user_stack::USER_STACK_RANDOM_BYTES];
    let mut stack_aslr = [0; super::user_stack::USER_STACK_ASLR_BYTES];
    at_random.copy_from_slice(&entropy[..super::user_stack::USER_STACK_RANDOM_BYTES]);
    stack_aslr.copy_from_slice(&entropy[super::user_stack::USER_STACK_RANDOM_BYTES..]);
    Ok(ExecEntropy {
        at_random,
        stack_aslr,
    })
}

#[cfg(any(app_smoke, app_user_boot))]
fn validate_exec_entropy_len(len: usize) -> Result<(), ExecError> {
    if len == super::user_stack::USER_STACK_ENTROPY_BYTES {
        Ok(())
    } else {
        Err(ExecError::EntropyUnavailable)
    }
}

#[cfg(app_user_boot)]
fn commit_prepared(
    ctx: &mut crate::context::Context,
    owner: ExecOwner,
    runtime_frame: Option<&mut super::trap_type::TrapFrame>,
    main_image: &[u8],
) -> Result<ExecSuccess, ExecError> {
    use super::exception_type as observation;
    use crate::checkpoint::Checkpoint;

    let retention = classify_retired_image_retention(ctx, owner).inspect_err(|_| {
        observe_failure(
            owner,
            observation::EXECVE_FAIL_STAGE_IMAGE_RETENTION,
            observation::EXECVE_FAIL_REASON_IMAGE_RETENTION,
            0,
            ctx,
        );
    })?;
    let builtin_subsequent_exec = retention == RetiredImageRetention::None
        && ctx
            .user_task_set
            .builtin_grandchild_exec_parent_snapshot_live();
    ctx.exec_transaction.mark_point_of_no_return()?;
    if !commit_runtime_flow_handoff(ctx) {
        exec_terminal("runtime exec flow handoff invariant failed\n");
    }
    let old_satp = crate::arch::riscv64::csr::read_satp();
    let new_satp = ctx.exec_transaction.staging_address_space.satp_token();
    let mut directly_released = 0;
    if builtin_subsequent_exec {
        directly_released += ctx
            .user_address_space
            .release_retired_exec_image(&mut ctx.page_allocator, &ctx.page_metadata_map);
        directly_released += ctx
            .user_stack
            .release_exec_backing(&mut ctx.page_allocator, &ctx.page_metadata_map);
    } else {
        unsafe {
            core::ptr::copy_nonoverlapping(
                &ctx.user_address_space,
                &mut ctx.exec_transaction.retired_address_space,
                1,
            );
        }
    }
    unsafe {
        core::ptr::copy_nonoverlapping(
            &ctx.exec_transaction.staging_address_space,
            &mut ctx.user_address_space,
            1,
        );
    }
    ctx.exec_transaction
        .staging_address_space
        .reset_staging_after_exec_commit();

    if owner == ExecOwner::Runtime {
        let report = ctx
            .files_struct
            .close_on_exec()
            .unwrap_or_else(|_| exec_terminal("exec CLOEXEC invariant failed\n"));
        observation::print_execve_close_on_exec_report(report);
        observation::record_execve_context_replaced(old_satp, new_satp);
        crate::checkpoint::dispatch(Checkpoint::UserExecContextReplaced, ctx);
        observation::record_execve_address_space(
            observation::EXECVE_OBS_STAGE_SATP_READY,
            &ctx.user_address_space,
        );
        crate::checkpoint::dispatch(Checkpoint::UserExecSatpReady, ctx);
    }

    if !builtin_subsequent_exec {
        core::mem::swap(&mut ctx.user_stack, &mut ctx.exec_transaction.retired_stack);
    }
    core::mem::swap(&mut ctx.user_stack, &mut ctx.exec_transaction.staging_stack);
    unsafe {
        core::ptr::copy_nonoverlapping(&ctx.exec_transaction.staging_elf, &mut ctx.elf_object, 1);
        core::ptr::copy_nonoverlapping(
            &ctx.exec_transaction.staging_interpreter,
            &mut ctx.elf_interpreter_object,
            1,
        );
        core::ptr::copy_nonoverlapping(
            &ctx.exec_transaction.staging_trap_frame,
            &mut ctx.user_trap_frame,
            1,
        );
    }
    ctx.exec_transaction
        .staging_stack
        .reset_staging_after_exec_commit();
    ctx.exec_transaction.staging_elf = ElfObject::new();
    ctx.exec_transaction.staging_interpreter = ElfObject::new();
    ctx.exec_transaction.staging_trap_frame = UserTrapFrame::new();

    match retention {
        RetiredImageRetention::BuiltinGrandchild => {
            if !ctx.user_task_set.retain_builtin_grandchild_exec_objects(
                &ctx.exec_transaction.retired_address_space,
                &mut ctx.exec_transaction.retired_stack,
            ) {
                exec_terminal("builtin grandchild exec retention invariant failed\n");
            }
        }
        RetiredImageRetention::OuterChild => {
            if !ctx.user_task_set.retain_parent_exec_objects(
                &ctx.exec_transaction.retired_address_space,
                &mut ctx.exec_transaction.retired_stack,
            ) {
                exec_terminal("outer child exec retention invariant failed\n");
            }
        }
        RetiredImageRetention::None => {}
    }

    if owner == ExecOwner::Runtime {
        observation::record_execve_trap_frame(&ctx.user_trap_frame);
        crate::checkpoint::dispatch(Checkpoint::UserExecTrapFrameReady, ctx);
        crate::arch::riscv64::csr::write_satp(new_satp);
        crate::arch::riscv64::csr::sfence_vma();
        observation::record_execve_stage(observation::EXECVE_OBS_STAGE_SATP_SWITCHED);
        crate::checkpoint::dispatch(Checkpoint::UserExecSatpSwitched, ctx);
        let frame = runtime_frame
            .unwrap_or_else(|| exec_terminal("runtime exec frame missing after commit\n"));
        reset_frame_for_exec_start(
            frame,
            ctx.user_trap_frame.entry(),
            ctx.user_trap_frame.sp(),
            ctx.user_trap_frame.sstatus(),
        );
        observation::record_execve_return_frame(frame);
        crate::checkpoint::dispatch(Checkpoint::UserExecReturnFrameReady, ctx);
    }

    let released = match retention {
        RetiredImageRetention::BuiltinGrandchild => 0,
        RetiredImageRetention::OuterChild => {
            ctx.exec_transaction
                .retired_address_space
                .reset_staging_after_exec_commit();
            ctx.exec_transaction
                .retired_stack
                .reset_staging_after_exec_commit();
            0
        }
        RetiredImageRetention::None if builtin_subsequent_exec => directly_released,
        RetiredImageRetention::None => {
            let mut released = ctx
                .exec_transaction
                .retired_address_space
                .release_retired_exec_image(&mut ctx.page_allocator, &ctx.page_metadata_map);
            released += ctx
                .exec_transaction
                .retired_stack
                .release_exec_backing(&mut ctx.page_allocator, &ctx.page_metadata_map);
            released
        }
    };
    if retention == RetiredImageRetention::None
        && ctx.user_task_set.builtin_grandchild_active()
        && !ctx
            .user_task_set
            .mark_builtin_grandchild_subsequent_exec_committed()
    {
        exec_terminal("builtin grandchild subsequent exec ownership invariant failed\n");
    }
    if ctx.user_task_set.builtin_grandchild_active() {
        observation::print_builtin_grandchild_exec_retention(
            match retention {
                RetiredImageRetention::None => "none",
                RetiredImageRetention::OuterChild => "outer_child",
                RetiredImageRetention::BuiltinGrandchild => "builtin_grandchild",
            },
            ctx.user_task_set.builtin_grandchild_exec_commit_count(),
            ctx.user_address_space.satp_token(),
            ctx.user_task_set.builtin_grandchild_parent_exec_satp(),
            ctx.user_task_set
                .builtin_grandchild_parent_exec_snapshot_saved(),
            ctx.user_task_set
                .builtin_grandchild_parent_exec_snapshot_restored(),
            released,
        );
    }
    ctx.exec_transaction.finish_commit(released);
    Ok(ExecSuccess {
        image_contains_stdin_fixture: super::elf_object::contains_bytes(
            main_image,
            b"user-smoke: begin",
        ),
        retired_pages_released: released,
    })
}

#[cfg(any(app_smoke, app_user_boot))]
fn classify_retired_image_retention(
    ctx: &crate::context::Context,
    owner: ExecOwner,
) -> Result<RetiredImageRetention, ExecError> {
    if owner != ExecOwner::Runtime {
        return Ok(RetiredImageRetention::None);
    }
    if ctx.user_task_set.builtin_grandchild_active() {
        if ctx
            .user_task_set
            .builtin_grandchild_first_exec_retention_required()
        {
            if ctx
                .user_task_set
                .can_retain_builtin_grandchild_exec_objects(
                    &ctx.user_address_space,
                    &ctx.user_stack,
                )
            {
                return Ok(RetiredImageRetention::BuiltinGrandchild);
            }
            return Err(ExecError::InvalidState);
        }
        if ctx
            .user_task_set
            .builtin_grandchild_exec_parent_snapshot_live()
        {
            return Ok(RetiredImageRetention::None);
        }
        return Err(ExecError::InvalidState);
    }
    if ctx
        .user_task_set
        .runtime_exec_parent_snapshot_live(&ctx.user_address_space)
    {
        if ctx
            .user_task_set
            .can_retain_parent_exec_objects(&ctx.user_address_space, &ctx.user_stack)
        {
            return Ok(RetiredImageRetention::OuterChild);
        }
        return Err(ExecError::InvalidState);
    }
    Ok(RetiredImageRetention::None)
}

#[cfg(app_user_boot)]
fn read_exec_image(
    ctx: &mut crate::context::Context,
    path: &[u8],
    interpreter: bool,
) -> Result<&'static [u8], ()> {
    let mut provider = super::virtio_blk::live_provider(&ctx.kernel_image);
    let buffer = unsafe {
        if interpreter {
            &mut *core::ptr::addr_of_mut!(EXEC_INTERPRETER_READ_BUFFER)
        } else {
            &mut *core::ptr::addr_of_mut!(EXEC_MAIN_READ_BUFFER)
        }
    };
    buffer.fill(0);
    let len = ctx
        .vfs_core
        .read_path(
            &ctx.fs_struct,
            &mut ctx.ext2_filesystem,
            &mut ctx.block_device_registry,
            &mut provider,
            path,
            buffer,
        )
        .map_err(|_| ())?;
    Ok(&buffer[..len])
}

#[cfg(app_user_boot)]
fn read_main_exec_image(ctx: &mut crate::context::Context) -> Result<&'static [u8], ()> {
    let path_ptr = ctx.exec_transaction.arguments.filename.as_ptr();
    let path_len = ctx.exec_transaction.arguments.filename_len;
    let path = unsafe { core::slice::from_raw_parts(path_ptr, path_len) };
    read_exec_image(ctx, path, false)
}

#[cfg(app_user_boot)]
fn format_error(error: BinaryFormatError, main: bool) -> (ExecError, usize, usize) {
    use super::exception_type as observation;
    match error {
        BinaryFormatError::NoExecutableFormat => (
            ExecError::NoExecutableFormat(None),
            if main {
                observation::EXECVE_FAIL_STAGE_MAIN_PRESET
            } else {
                observation::EXECVE_FAIL_STAGE_INTERPRETER_PRESET
            },
            0,
        ),
        BinaryFormatError::InvalidElf(stage, error) => (
            elf_exec_error(error),
            match (main, stage) {
                (true, ElfPreparationStage::Preset) => observation::EXECVE_FAIL_STAGE_MAIN_PRESET,
                (true, ElfPreparationStage::Setup) => observation::EXECVE_FAIL_STAGE_MAIN_SETUP,
                (false, ElfPreparationStage::Preset) => {
                    observation::EXECVE_FAIL_STAGE_INTERPRETER_PRESET
                }
                (false, ElfPreparationStage::Setup) => {
                    observation::EXECVE_FAIL_STAGE_INTERPRETER_SETUP
                }
            },
            error.index(),
        ),
    }
}

#[cfg(app_user_boot)]
fn elf_exec_error(error: ElfError) -> ExecError {
    match error {
        ElfError::BackingAllocationFailed
        | ElfError::PageTableAllocationFailed
        | ElfError::TooManyMappingPages => ExecError::NoMemory,
        _ => ExecError::NoExecutableFormat(Some(error)),
    }
}

#[cfg(app_user_boot)]
fn observe_failure(
    owner: ExecOwner,
    stage: usize,
    reason: usize,
    detail: usize,
    ctx: &crate::context::Context,
) {
    if owner == ExecOwner::Runtime {
        super::exception_type::record_execve_failure_detail(
            stage,
            reason,
            detail,
            ctx.page_allocator.buddy_total_free_pages(),
            ctx.page_allocator.totalram_pages(),
        );
    }
}

#[cfg(app_user_boot)]
fn reset_frame_for_exec_start(
    frame: &mut super::trap_type::TrapFrame,
    entry: usize,
    sp: usize,
    sstatus: usize,
) {
    let mut index = 1usize;
    while index < 32 {
        frame.set_reg(index, 0);
        index += 1;
    }
    frame.set_reg(2, sp);
    frame.sstatus = sstatus;
    frame.sepc = entry;
    frame.stval = 0;
}

fn exec_terminal(message: &str) -> ! {
    crate::arch::riscv64::sbi::putstr(message);
    crate::arch::riscv64::sbi::system_shutdown()
}
