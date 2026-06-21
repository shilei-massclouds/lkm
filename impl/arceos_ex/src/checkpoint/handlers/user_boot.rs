use crate::{
    checkpoint::{
        handlers::{CheckpointOutcome, Handler, HandlerRun, HandlerScope},
        kunit::Sink,
    },
    context::Context,
    objects::{
        state::State,
        user_boot::{
            ElfError, ElfObject, UserAddressSpace, UserStack, UserTrapFrame,
            USER_INIT_EXPECTED_MESSAGE,
        },
    },
    trace::Checkpoint,
};

const SCOPE: &[Checkpoint] = &[
    Checkpoint::PayloadPhaseOnline,
    #[cfg(app_user_boot)]
    Checkpoint::UserModeEntry,
    #[cfg(app_user_boot)]
    Checkpoint::SyscallTableWrite,
    #[cfg(app_user_boot)]
    Checkpoint::SyscallTableExit,
];
#[cfg(app_user_boot)]
pub const KUNIT_CASE_COUNT: usize = 9;
#[cfg(not(app_user_boot))]
pub const KUNIT_CASE_COUNT: usize = 6;

pub const HANDLER: Handler = Handler {
    name: "user_boot.elf_parser",
    priority: 120,
    scope: HandlerScope::Only(SCOPE),
    run: HandlerRun::Observe(run),
};

fn run(checkpoint: Checkpoint, ctx: &Context, sink: &mut dyn Sink) -> CheckpointOutcome {
    let total = super::kunit_case_count();
    match checkpoint {
        Checkpoint::PayloadPhaseOnline => {
            run_valid_fixture(checkpoint, sink, total);
            run_bad_magic(checkpoint, sink, total);
            run_wrong_machine(checkpoint, sink, total);
            run_invalid_segment(checkpoint, sink, total);
            run_bss_plan(checkpoint, sink, total);
            run_trap_frame_rejects_unready_inputs(checkpoint, sink, total);
        }
        #[cfg(app_user_boot)]
        Checkpoint::UserModeEntry => {
            run_user_mode_entry(checkpoint, ctx, sink, total);
        }
        #[cfg(app_user_boot)]
        Checkpoint::SyscallTableWrite => {
            run_syscall_table_write(checkpoint, ctx, sink, total);
        }
        #[cfg(app_user_boot)]
        Checkpoint::SyscallTableExit => {
            run_syscall_table_exit(checkpoint, ctx, sink, total);
        }
        _ => {}
    }
    CheckpointOutcome::Continue
}

#[cfg(app_user_boot)]
fn run_user_mode_entry(checkpoint: Checkpoint, ctx: &Context, sink: &mut dyn Sink, total: usize) {
    let name = "user_boot.user_init_process.enter_user_mode";
    sink.start_case(total, "", name, checkpoint);

    let process = &ctx.user_init_process;
    let valid = process.state() == State::Online
        && process.reuses_kernel_init_task()
        && process.pid1_preserved()
        && process.exec_identity_handoff()
        && process.address_space_bound()
        && process.fs_struct_inherited()
        && process.trap_frame_bound()
        && process.syscall_context_bound()
        && process.trap_return_bound()
        && process.user_entry_ready()
        && process.runtime_entered()
        && ctx.user_address_space.state() == State::Online
        && ctx.user_address_space.runtime_ready()
        && ctx.user_trap_frame.state() == State::Ready
        && ctx.user_trap_frame.sret_ready();

    sink.diag_usize(
        "user_init_runtime_entered",
        process.runtime_entered() as usize,
    );
    sink.diag_usize("user_entry_ready", process.user_entry_ready() as usize);
    sink.diag_usize(
        "user_address_space_runtime_ready",
        ctx.user_address_space.runtime_ready() as usize,
    );
    if valid {
        sink.pass(total, "", name);
    } else {
        sink.fail(total, "", name, "user mode entry facts invalid");
    }
}

#[cfg(app_user_boot)]
fn run_syscall_table_write(
    checkpoint: Checkpoint,
    ctx: &Context,
    sink: &mut dyn Sink,
    total: usize,
) {
    let name = "user_boot.syscall_table.write";
    sink.start_case(total, "", name, checkpoint);

    let process = &ctx.user_init_process;
    let table = &ctx.syscall_table;
    let valid = process.state() == State::Online
        && process.runtime_entered()
        && process.syscall_context_bound()
        && process.syscall_dispatch_bound()
        && process.syscall_arguments_extracted()
        && table.state() == State::Ready
        && table.bound_to_exception()
        && table.write_supported()
        && table.write_usercopy_ready()
        && table.write_routes_to_console()
        && table.write_observed()
        && ctx.exception_stream.syscall_state() == State::Online;

    sink.diag_usize(
        "syscall_table_write_observed",
        table.write_observed() as usize,
    );
    sink.diag_usize(
        "syscall_table_write_supported",
        table.write_supported() as usize,
    );
    if valid {
        sink.pass(total, "", name);
    } else {
        sink.fail(total, "", name, "syscall table write facts invalid");
    }
}

#[cfg(app_user_boot)]
fn run_syscall_table_exit(
    checkpoint: Checkpoint,
    ctx: &Context,
    sink: &mut dyn Sink,
    total: usize,
) {
    let name = "user_boot.syscall_table.exit";
    sink.start_case(total, "", name, checkpoint);

    let process = &ctx.user_init_process;
    let table = &ctx.syscall_table;
    let valid = process.state() == State::Online
        && process.runtime_entered()
        && process.syscall_context_bound()
        && table.state() == State::Ready
        && table.bound_to_exception()
        && table.exit_supported()
        && table.exit_group_supported()
        && table.exit_records_status()
        && table.write_observed()
        && table.exit_observed()
        && ctx.exception_stream.syscall_state() == State::Online;

    sink.diag_usize(
        "syscall_table_write_observed",
        table.write_observed() as usize,
    );
    sink.diag_usize(
        "syscall_table_exit_observed",
        table.exit_observed() as usize,
    );
    sink.diag_usize(
        "syscall_table_exit_supported",
        table.exit_supported() as usize,
    );
    sink.diag_usize(
        "syscall_table_exit_group_supported",
        table.exit_group_supported() as usize,
    );
    if valid {
        sink.pass(total, "", name);
    } else {
        sink.fail(total, "", name, "syscall table exit facts invalid");
    }
}

fn run_valid_fixture(checkpoint: Checkpoint, sink: &mut dyn Sink, total: usize) {
    let name = "user_boot.elf_parser.valid_fixture";
    sink.start_case(total, "", name, checkpoint);

    let mut image = FixtureElf::new();
    image.write_supported_header();
    image.write_load_segment(0, 0, 0x10000, image.len(), image.len(), 5, 0x1000);
    image.write_message(0x80);

    let mut elf = ElfObject::new();
    if elf.preset_from_vfs(image.as_slice()).is_err() || elf.setup(image.as_slice()).is_err() {
        sink.fail(total, "", name, "valid fixture ELF did not parse");
        return;
    }
    let Some(segment) = elf.load_segment(0) else {
        sink.fail(total, "", name, "load segment missing");
        return;
    };

    let valid = elf.input_bound()
        && elf.input_from_vfs()
        && elf.magic_valid()
        && elf.class_elf64()
        && elf.little_endian()
        && elf.machine_riscv()
        && elf.type_supported()
        && elf.static_executable()
        && elf.no_separate_loader()
        && elf.program_headers_parsed()
        && elf.pt_load_segments_bound()
        && elf.segment_permissions_bound()
        && elf.load_plan_bound()
        && elf.entry_in_executable_segment()
        && elf.init_content_observed()
        && elf.bss_zero_plan_bound()
        && elf.entry_bound()
        && elf.load_merged_into_setup()
        && elf.entry() == 0x10000
        && elf.program_header_count() == 1
        && elf.load_segment_count() == 1
        && segment.offset() == 0
        && segment.vaddr() == 0x10000
        && segment.filesz() == image.len()
        && segment.memsz() == image.len()
        && segment.readable()
        && segment.executable()
        && !segment.writable()
        && segment.align() == 0x1000;

    if !valid {
        sink.fail(total, "", name, "valid fixture ELF facts invalid");
        return;
    }

    sink.diag_usize("elf_entry", elf.entry());
    sink.diag_usize("elf_load_segments", elf.load_segment_count());
    sink.pass(total, "", name);
}

fn run_bad_magic(checkpoint: Checkpoint, sink: &mut dyn Sink, total: usize) {
    let name = "user_boot.elf_parser.bad_magic";
    sink.start_case(total, "", name, checkpoint);

    let mut image = FixtureElf::new();
    image.write_supported_header();
    image.as_mut_slice()[0] = 0;
    let mut elf = ElfObject::new();
    match elf.preset_from_vfs(image.as_slice()) {
        Err(ElfError::BadMagic) => sink.pass(total, "", name),
        _ => sink.fail(total, "", name, "bad magic accepted"),
    }
}

fn run_wrong_machine(checkpoint: Checkpoint, sink: &mut dyn Sink, total: usize) {
    let name = "user_boot.elf_parser.wrong_machine";
    sink.start_case(total, "", name, checkpoint);

    let mut image = FixtureElf::new();
    image.write_supported_header();
    image.write_u16(18, 62);
    let mut elf = ElfObject::new();
    match elf.preset_from_vfs(image.as_slice()) {
        Err(ElfError::UnsupportedMachine) => sink.pass(total, "", name),
        _ => sink.fail(total, "", name, "wrong machine accepted"),
    }
}

fn run_invalid_segment(checkpoint: Checkpoint, sink: &mut dyn Sink, total: usize) {
    let name = "user_boot.elf_parser.invalid_segment";
    sink.start_case(total, "", name, checkpoint);

    let mut image = FixtureElf::new();
    image.write_supported_header();
    image.write_load_segment(0, 0, 0x10000, image.len(), image.len() - 1, 5, 0x1000);
    image.write_message(0x80);
    let mut elf = ElfObject::new();
    if elf.preset_from_vfs(image.as_slice()).is_err() {
        sink.fail(
            total,
            "",
            name,
            "fixture preset failed before segment check",
        );
        return;
    }

    match elf.setup(image.as_slice()) {
        Err(ElfError::InvalidProgramHeader) => sink.pass(total, "", name),
        _ => sink.fail(total, "", name, "invalid segment accepted"),
    }
}

fn run_bss_plan(checkpoint: Checkpoint, sink: &mut dyn Sink, total: usize) {
    let name = "user_boot.elf_parser.bss_plan";
    sink.start_case(total, "", name, checkpoint);

    let mut image = FixtureElf::new();
    image.write_supported_header();
    image.write_load_segment(0, 0, 0x10000, 256, 512, 5, 0x1000);
    image.write_message(0x80);

    let mut elf = ElfObject::new();
    if elf.preset_from_vfs(image.as_slice()).is_err() || elf.setup(image.as_slice()).is_err() {
        sink.fail(total, "", name, "fixture ELF did not parse");
        return;
    }
    let Some(segment) = elf.load_segment(0) else {
        sink.fail(total, "", name, "load segment missing");
        return;
    };

    if elf.bss_zero_plan_bound()
        && segment.filesz() == 256
        && segment.memsz() == 512
        && segment.memsz() - segment.filesz() == 256
    {
        sink.pass(total, "", name);
    } else {
        sink.fail(total, "", name, "bss plan facts invalid");
    }
}

fn run_trap_frame_rejects_unready_inputs(
    checkpoint: Checkpoint,
    sink: &mut dyn Sink,
    total: usize,
) {
    let name = "user_boot.trap_frame.rejects_unready_inputs";
    sink.start_case(total, "", name, checkpoint);

    let mut image = FixtureElf::new();
    image.write_supported_header();
    image.write_load_segment(0, 0, 0x10000, image.len(), image.len(), 5, 0x1000);
    image.write_message(0x80);

    let mut elf = ElfObject::new();
    if elf.preset_from_vfs(image.as_slice()).is_err() || elf.setup(image.as_slice()).is_err() {
        sink.fail(total, "", name, "fixture ELF did not parse");
        return;
    }

    let address_space = UserAddressSpace::new();
    let stack = UserStack::new();
    let mut frame = UserTrapFrame::new();
    if frame.setup(&address_space, &elf, &stack).is_err() {
        sink.pass(total, "", name);
    } else {
        sink.fail(total, "", name, "unready inputs accepted");
    }
}

struct FixtureElf {
    bytes: [u8; 256],
}

impl FixtureElf {
    const fn new() -> Self {
        Self { bytes: [0; 256] }
    }

    const fn len(&self) -> usize {
        self.bytes.len()
    }

    fn as_slice(&self) -> &[u8] {
        &self.bytes
    }

    fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.bytes
    }

    fn write_supported_header(&mut self) {
        self.bytes[0..4].copy_from_slice(b"\x7fELF");
        self.bytes[4] = 2;
        self.bytes[5] = 1;
        self.bytes[6] = 1;
        self.write_u16(16, 2);
        self.write_u16(18, 243);
        self.write_u32(20, 1);
        self.write_u64(24, 0x10000);
        self.write_u64(32, 64);
        self.write_u16(52, 64);
        self.write_u16(54, 56);
        self.write_u16(56, 1);
    }

    fn write_load_segment(
        &mut self,
        index: usize,
        offset: usize,
        vaddr: usize,
        filesz: usize,
        memsz: usize,
        flags: u32,
        align: usize,
    ) {
        let phdr = 64 + index * 56;
        self.write_u32(phdr, 1);
        self.write_u32(phdr + 4, flags);
        self.write_u64(phdr + 8, offset as u64);
        self.write_u64(phdr + 16, vaddr as u64);
        self.write_u64(phdr + 24, vaddr as u64);
        self.write_u64(phdr + 32, filesz as u64);
        self.write_u64(phdr + 40, memsz as u64);
        self.write_u64(phdr + 48, align as u64);
    }

    fn write_message(&mut self, offset: usize) {
        self.bytes[offset..offset + USER_INIT_EXPECTED_MESSAGE.len()]
            .copy_from_slice(USER_INIT_EXPECTED_MESSAGE);
    }

    fn write_u16(&mut self, offset: usize, value: u16) {
        self.bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn write_u32(&mut self, offset: usize, value: u32) {
        self.bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn write_u64(&mut self, offset: usize, value: u64) {
        self.bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }
}
