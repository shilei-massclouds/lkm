use super::{
    block_device::{
        BlockDeviceError, BlockDeviceProvider, BlockDeviceRef, BlockDeviceRegistry, DevT,
    },
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
};

pub const BUFFER_HEAD_SECTOR_SIZE: usize = 512;

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum BioOp {
    Read,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum BioSubmitPath {
    DefaultBlockDevice,
    MajorMinorLookup,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum BlockIoError {
    Registry(BlockDeviceError),
    DeviceMissing,
    DeviceNotReady,
    InvalidBlockSize,
    ShortRead,
    EmptyRead,
}

impl From<BlockDeviceError> for BlockIoError {
    fn from(error: BlockDeviceError) -> Self {
        Self::Registry(error)
    }
}

pub struct Bio {
    lifecycle: Lifecycle,
    op: BioOp,
    submit_path: BioSubmitPath,
    device_ref: BlockDeviceRef,
    devt: DevT,
    sector: u64,
    len: usize,
    buffer_bound: bool,
    submit_bio_wait_called: bool,
    blk_mq_submit_bio_entered: bool,
    submitted: bool,
    completion_observed: bool,
    status_ok: bool,
    copies_to_caller: bool,
}

impl Bio {
    fn new_read(
        device_ref: BlockDeviceRef,
        devt: DevT,
        sector: u64,
        len: usize,
        submit_path: BioSubmitPath,
    ) -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            op: BioOp::Read,
            submit_path,
            device_ref,
            devt,
            sector,
            len,
            buffer_bound: len != 0,
            submit_bio_wait_called: false,
            blk_mq_submit_bio_entered: false,
            submitted: false,
            completion_observed: false,
            status_ok: false,
            copies_to_caller: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn op(&self) -> BioOp {
        self.op
    }

    pub const fn submit_path(&self) -> BioSubmitPath {
        self.submit_path
    }

    pub const fn device_ref(&self) -> BlockDeviceRef {
        self.device_ref
    }

    pub const fn devt(&self) -> DevT {
        self.devt
    }

    pub const fn sector(&self) -> u64 {
        self.sector
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn buffer_bound(&self) -> bool {
        self.buffer_bound
    }

    pub const fn submit_bio_wait_called(&self) -> bool {
        self.submit_bio_wait_called
    }

    pub const fn blk_mq_submit_bio_entered(&self) -> bool {
        self.blk_mq_submit_bio_entered
    }

    pub const fn submitted(&self) -> bool {
        self.submitted
    }

    pub const fn completion_observed(&self) -> bool {
        self.completion_observed
    }

    pub const fn status_ok(&self) -> bool {
        self.status_ok
    }

    pub const fn copies_to_caller(&self) -> bool {
        self.copies_to_caller
    }

    fn setup(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Base || !self.buffer_bound {
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

    fn mark_submitted(&mut self) {
        self.submit_bio_wait_called = true;
        self.blk_mq_submit_bio_entered = true;
        self.submitted = true;
        self.completion_observed = true;
        self.status_ok = true;
        self.copies_to_caller = true;
    }
}

pub struct BufferHead {
    lifecycle: Lifecycle,
    device_ref: BlockDeviceRef,
    devt: DevT,
    sector: u64,
    block_size: usize,
    data: [u8; BUFFER_HEAD_SECTOR_SIZE],
    len: usize,
    bio: Bio,
    sb_bread_called: bool,
    bread_gfp_called: bool,
    uptodate: bool,
    data_nonzero: bool,
}

impl BufferHead {
    fn new(
        device_ref: BlockDeviceRef,
        devt: DevT,
        sector: u64,
        submit_path: BioSubmitPath,
    ) -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            device_ref,
            devt,
            sector,
            block_size: BUFFER_HEAD_SECTOR_SIZE,
            data: [0; BUFFER_HEAD_SECTOR_SIZE],
            len: 0,
            bio: Bio::new_read(
                device_ref,
                devt,
                sector,
                BUFFER_HEAD_SECTOR_SIZE,
                submit_path,
            ),
            sb_bread_called: false,
            bread_gfp_called: false,
            uptodate: false,
            data_nonzero: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn device_ref(&self) -> BlockDeviceRef {
        self.device_ref
    }

    pub const fn devt(&self) -> DevT {
        self.devt
    }

    pub const fn sector(&self) -> u64 {
        self.sector
    }

    pub const fn block_size(&self) -> usize {
        self.block_size
    }

    pub fn data(&self) -> &[u8] {
        &self.data[..self.len]
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn bio(&self) -> &Bio {
        &self.bio
    }

    pub const fn sb_bread_called(&self) -> bool {
        self.sb_bread_called
    }

    pub const fn bread_gfp_called(&self) -> bool {
        self.bread_gfp_called
    }

    pub const fn submit_bio_wait_used(&self) -> bool {
        self.bio.submit_bio_wait_called()
    }

    pub const fn uptodate(&self) -> bool {
        self.uptodate
    }

    pub const fn data_nonzero(&self) -> bool {
        self.data_nonzero
    }

    fn setup(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Base || self.block_size == 0 {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.bio.setup()?;
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }

    fn mark_read_complete(&mut self, len: usize) -> Result<(), BlockIoError> {
        if len != self.block_size {
            return Err(BlockIoError::ShortRead);
        }
        let nonzero = self.data[..len].iter().any(|byte| *byte != 0);
        if !nonzero {
            return Err(BlockIoError::EmptyRead);
        }

        self.len = len;
        self.sb_bread_called = true;
        self.bread_gfp_called = true;
        self.uptodate = true;
        self.data_nonzero = true;
        Ok(())
    }
}

pub fn sb_bread_default<P: BlockDeviceProvider>(
    registry: &mut BlockDeviceRegistry,
    provider: &mut P,
    sector: u64,
) -> Result<BufferHead, BlockIoError> {
    let Some(entry) = registry.default_entry() else {
        return Err(BlockIoError::DeviceMissing);
    };
    let device_ref = entry.device_ref();
    let devt = entry.devt();
    let block_size = entry.sector_size() as usize;
    if block_size != BUFFER_HEAD_SECTOR_SIZE {
        return Err(BlockIoError::InvalidBlockSize);
    }

    let mut bh = BufferHead::new(device_ref, devt, sector, BioSubmitPath::DefaultBlockDevice);
    bh.setup().map_err(|_| BlockIoError::DeviceNotReady)?;
    let len = submit_bio_wait_default(registry, provider, &mut bh.bio, sector, &mut bh.data)?;
    bh.mark_read_complete(len)?;
    Ok(bh)
}

pub fn sb_bread_by_devt<P: BlockDeviceProvider>(
    registry: &mut BlockDeviceRegistry,
    provider: &mut P,
    devt: DevT,
    sector: u64,
) -> Result<BufferHead, BlockIoError> {
    let Some(entry) = registry.lookup(devt) else {
        return Err(BlockIoError::DeviceMissing);
    };
    let device_ref = entry.device_ref();
    let block_size = entry.sector_size() as usize;
    if block_size != BUFFER_HEAD_SECTOR_SIZE {
        return Err(BlockIoError::InvalidBlockSize);
    }

    let mut bh = BufferHead::new(device_ref, devt, sector, BioSubmitPath::MajorMinorLookup);
    bh.setup().map_err(|_| BlockIoError::DeviceNotReady)?;
    let len = submit_bio_wait_by_devt(registry, provider, &mut bh.bio, devt, sector, &mut bh.data)?;
    bh.mark_read_complete(len)?;
    Ok(bh)
}

fn submit_bio_wait_default<P: BlockDeviceProvider>(
    registry: &mut BlockDeviceRegistry,
    provider: &mut P,
    bio: &mut Bio,
    sector: u64,
    buffer: &mut [u8],
) -> Result<usize, BlockIoError> {
    if bio.state() != State::Ready
        || bio.op() != BioOp::Read
        || bio.submit_path() != BioSubmitPath::DefaultBlockDevice
    {
        return Err(BlockIoError::Registry(BlockDeviceError::DeviceNotReady));
    }
    let len = registry.read_default(provider, sector, buffer)?;
    bio.mark_submitted();
    Ok(len)
}

fn submit_bio_wait_by_devt<P: BlockDeviceProvider>(
    registry: &mut BlockDeviceRegistry,
    provider: &mut P,
    bio: &mut Bio,
    devt: DevT,
    sector: u64,
    buffer: &mut [u8],
) -> Result<usize, BlockIoError> {
    if bio.state() != State::Ready
        || bio.op() != BioOp::Read
        || bio.submit_path() != BioSubmitPath::MajorMinorLookup
    {
        return Err(BlockIoError::Registry(BlockDeviceError::DeviceNotReady));
    }
    let len = registry.read_by_devt(provider, devt, sector, buffer)?;
    bio.mark_submitted();
    Ok(len)
}
