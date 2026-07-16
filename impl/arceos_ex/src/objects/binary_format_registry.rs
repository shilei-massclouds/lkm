use super::{
    elf_object::{ElfError, ElfObject},
    initcall::InitcallTable,
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
};

const BINARY_FORMAT_HANDLER_CAPACITY: usize = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BinaryFormatHandler {
    Elf(ElfBinaryFormat),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ElfBinaryFormat;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ElfPreparationStage {
    Preset,
    Setup,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BinaryFormatError {
    NoExecutableFormat,
    InvalidElf(ElfPreparationStage, ElfError),
}

pub struct BinaryFormatRegistry {
    lifecycle: Lifecycle,
    handlers: [Option<BinaryFormatHandler>; BINARY_FORMAT_HANDLER_CAPACITY],
    handler_count: usize,
    script_deferred: bool,
    misc_deferred: bool,
    dynamic_registration_deferred: bool,
    module_retry_trimmed: bool,
}

#[allow(dead_code)]
impl BinaryFormatRegistry {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            handlers: [None; BINARY_FORMAT_HANDLER_CAPACITY],
            handler_count: 0,
            script_deferred: false,
            misc_deferred: false,
            dynamic_registration_deferred: false,
            module_retry_trimmed: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn handler_count(&self) -> usize {
        self.handler_count
    }

    pub const fn only_elf_handler(&self) -> bool {
        self.handler_count == 1
            && matches!(
                self.handlers[0],
                Some(BinaryFormatHandler::Elf(ElfBinaryFormat))
            )
    }

    pub const fn script_deferred(&self) -> bool {
        self.script_deferred
    }

    pub const fn misc_deferred(&self) -> bool {
        self.misc_deferred
    }

    pub const fn dynamic_registration_deferred(&self) -> bool {
        self.dynamic_registration_deferred
    }

    pub const fn module_retry_trimmed(&self) -> bool {
        self.module_retry_trimmed
    }

    pub fn setup(&mut self, initcall_table: &InitcallTable) -> EventResult {
        if self.lifecycle.state() != State::Base || initcall_table.state() != State::Ready {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.handlers[0] = Some(BinaryFormatHandler::Elf(ElfBinaryFormat));
        self.handler_count = 1;
        self.script_deferred = true;
        self.misc_deferred = true;
        self.dynamic_registration_deferred = true;
        self.module_retry_trimmed = true;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }

    pub fn prepare_main(&self, image: &[u8], elf: &mut ElfObject) -> Result<(), BinaryFormatError> {
        self.require_elf_handler(image)?;
        elf.preset_from_vfs(image)
            .map_err(|error| BinaryFormatError::InvalidElf(ElfPreparationStage::Preset, error))?;
        elf.setup(image)
            .map_err(|error| BinaryFormatError::InvalidElf(ElfPreparationStage::Setup, error))
    }

    pub fn prepare_interpreter(
        &self,
        image: &[u8],
        elf: &mut ElfObject,
    ) -> Result<(), BinaryFormatError> {
        self.require_elf_handler(image)?;
        elf.preset_interpreter_from_vfs(image)
            .map_err(|error| BinaryFormatError::InvalidElf(ElfPreparationStage::Preset, error))?;
        elf.setup(image)
            .map_err(|error| BinaryFormatError::InvalidElf(ElfPreparationStage::Setup, error))
    }

    fn require_elf_handler(&self, image: &[u8]) -> Result<(), BinaryFormatError> {
        if self.lifecycle.state() != State::Ready || !self.only_elf_handler() {
            return Err(BinaryFormatError::NoExecutableFormat);
        }
        if image.get(..4) != Some(b"\x7fELF") {
            return Err(BinaryFormatError::NoExecutableFormat);
        }
        Ok(())
    }
}
