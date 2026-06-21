use crate::{
    checkpoint::{
        handlers::{CheckpointOutcome, Handler, HandlerRun, HandlerScope},
        kunit::Sink,
    },
    context::Context,
    objects::{
        state::State,
        user_boot::{
            ElfError, ElfObject, UserAddressSpace, UserMappingKind, UserStack,
            USER_INIT_EXPECTED_MESSAGE, USER_STACK_SIZE, USER_STACK_TOP,
        },
    },
    trace::Checkpoint,
};

const SCOPE: &[Checkpoint] = &[Checkpoint::PayloadPhaseOnline];
pub const KUNIT_CASE_COUNT: usize = 6;

pub const HANDLER: Handler = Handler {
    name: "user_boot.elf_parser",
    priority: 120,
    scope: HandlerScope::Only(SCOPE),
    run: HandlerRun::Observe(run),
};

fn run(checkpoint: Checkpoint, ctx: &Context, sink: &mut dyn Sink) -> CheckpointOutcome {
    let total = super::kunit_case_count();
    run_valid_fixture(checkpoint, sink, total);
    run_bad_magic(checkpoint, sink, total);
    run_wrong_machine(checkpoint, sink, total);
    run_invalid_segment(checkpoint, sink, total);
    run_address_space_mapping(checkpoint, ctx, sink, total);
    run_address_space_requires_ready_elf(checkpoint, ctx, sink, total);
    CheckpointOutcome::Continue
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

fn run_address_space_mapping(
    checkpoint: Checkpoint,
    ctx: &Context,
    sink: &mut dyn Sink,
    total: usize,
) {
    let name = "user_boot.address_space.mapping_plan";
    sink.start_case(total, "", name, checkpoint);

    let Some((elf, mut address_space, stack)) = ready_mapping_fixture(ctx) else {
        sink.fail(total, "", name, "fixture setup failed");
        return;
    };

    if address_space.setup(&elf, &stack).is_err() {
        sink.fail(total, "", name, "address space setup failed");
        return;
    }

    let Some(first_mapping) = address_space.mapping(0) else {
        sink.fail(total, "", name, "first mapping missing");
        return;
    };
    let Some(stack_mapping) = address_space.stack_mapping() else {
        sink.fail(total, "", name, "stack mapping missing");
        return;
    };

    let valid = address_space.state() == State::Ready
        && address_space.allocated()
        && address_space.low_half_private()
        && address_space.high_half_shares_swapper()
        && address_space.kernel_pages_u_disabled()
        && address_space.user_pages_u_enabled()
        && address_space.elf_load_plan_consumed()
        && address_space.segment_mappings_bound()
        && address_space.entry_mapping_executable()
        && address_space.bss_zero_plan_consumed()
        && address_space.elf_segments_mapped()
        && address_space.stack_mapped()
        && address_space.elf_mapped()
        && address_space.elf_bss_zeroed()
        && !address_space.runtime_ready()
        && address_space.segment_mapping_count() == 1
        && address_space.mapping_count() == 2
        && first_mapping.kind() == UserMappingKind::ElfSegment
        && first_mapping.vaddr() == 0x10000
        && first_mapping.filesz() == 256
        && first_mapping.memsz() == 512
        && first_mapping.readable()
        && first_mapping.executable()
        && first_mapping.user_accessible()
        && first_mapping.bss_zero_bytes() == 256
        && stack.state() == State::Ready
        && stack.size() == USER_STACK_SIZE
        && stack.top() == USER_STACK_TOP
        && stack_mapping.kind() == UserMappingKind::Stack
        && stack_mapping.vaddr() == stack.base()
        && stack_mapping.memsz() == USER_STACK_SIZE
        && stack_mapping.writable()
        && !stack_mapping.executable();

    if !valid {
        sink.fail(total, "", name, "address space facts invalid");
        return;
    }

    sink.diag_usize("user_mappings", address_space.mapping_count());
    sink.pass(total, "", name);
}

fn run_address_space_requires_ready_elf(
    checkpoint: Checkpoint,
    ctx: &Context,
    sink: &mut dyn Sink,
    total: usize,
) {
    let name = "user_boot.address_space.requires_ready_elf";
    sink.start_case(total, "", name, checkpoint);

    let mut address_space = UserAddressSpace::new();
    if address_space
        .preset(
            ctx.vm.swapper_vm(),
            &ctx.page_allocator,
            &ctx.kernel_global_allocator,
        )
        .is_err()
    {
        sink.fail(total, "", name, "address space preset failed");
        return;
    }
    let mut stack = UserStack::new();
    if stack.setup(&address_space, &ctx.page_allocator).is_err() {
        sink.fail(total, "", name, "stack setup failed");
        return;
    }
    let elf = ElfObject::new();

    match address_space.setup(&elf, &stack) {
        Err(ElfError::InvalidState) => sink.pass(total, "", name),
        _ => sink.fail(total, "", name, "unready ELF accepted"),
    }
}

fn ready_mapping_fixture(ctx: &Context) -> Option<(ElfObject, UserAddressSpace, UserStack)> {
    let mut image = FixtureElf::new();
    image.write_supported_header();
    image.write_load_segment(0, 0, 0x10000, 256, 512, 5, 0x1000);
    image.write_message(0x80);

    let mut elf = ElfObject::new();
    elf.preset_from_vfs(image.as_slice()).ok()?;
    elf.setup(image.as_slice()).ok()?;

    let mut address_space = UserAddressSpace::new();
    address_space
        .preset(
            ctx.vm.swapper_vm(),
            &ctx.page_allocator,
            &ctx.kernel_global_allocator,
        )
        .ok()?;

    let mut stack = UserStack::new();
    stack.setup(&address_space, &ctx.page_allocator).ok()?;

    Some((elf, address_space, stack))
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
