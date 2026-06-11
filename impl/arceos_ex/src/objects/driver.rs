use super::{
    device::DeviceRef,
    device_tree::{DeviceNodeId, DeviceTree},
};

pub type PlatformProbe = fn(&DeviceTree, DeviceRef, DeviceNodeId) -> ProbeResult;

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum ProbeResult {
    Bound,
    Deferred,
}

pub struct OfMatchEntry {
    compatible: &'static [u8],
}

impl OfMatchEntry {
    pub const fn new(compatible: &'static [u8]) -> Self {
        Self { compatible }
    }

    pub const fn compatible(&self) -> &'static [u8] {
        self.compatible
    }
}

pub struct OfMatchTable {
    entries: &'static [OfMatchEntry],
}

impl OfMatchTable {
    pub const fn new(entries: &'static [OfMatchEntry]) -> Self {
        Self { entries }
    }

    pub const fn empty() -> Self {
        Self { entries: &[] }
    }

    pub fn matches(&self, compatible: &[u8]) -> bool {
        let mut index = 0usize;
        while index < self.entries.len() {
            if self.entries[index].compatible() == compatible {
                return true;
            }
            index += 1;
        }
        false
    }
}

pub struct DeviceDriver {
    name: &'static str,
    of_match_table: OfMatchTable,
}

impl DeviceDriver {
    pub const fn new(name: &'static str, of_match_table: OfMatchTable) -> Self {
        Self {
            name,
            of_match_table,
        }
    }

    pub const fn name(&self) -> &'static str {
        self.name
    }

    pub const fn of_match_table(&self) -> &OfMatchTable {
        &self.of_match_table
    }
}

pub struct PlatformDriver {
    driver: DeviceDriver,
    probe: PlatformProbe,
}

impl PlatformDriver {
    pub const fn new(
        name: &'static str,
        of_match_table: OfMatchTable,
        probe: PlatformProbe,
    ) -> Self {
        Self {
            driver: DeviceDriver::new(name, of_match_table),
            probe,
        }
    }

    pub const fn driver(&self) -> &DeviceDriver {
        &self.driver
    }

    pub fn probe(
        &self,
        device_tree: &DeviceTree,
        device: DeviceRef,
        node_id: DeviceNodeId,
    ) -> ProbeResult {
        (self.probe)(device_tree, device, node_id)
    }
}

#[derive(Clone, Copy)]
pub struct DeviceDriverRef {
    driver: &'static PlatformDriver,
}

impl DeviceDriverRef {
    pub const fn new(driver: &'static PlatformDriver) -> Self {
        Self { driver }
    }

    pub const fn driver(self) -> &'static PlatformDriver {
        self.driver
    }
}

impl PartialEq for DeviceDriverRef {
    fn eq(&self, other: &Self) -> bool {
        core::ptr::eq(self.driver, other.driver)
    }
}

impl Eq for DeviceDriverRef {}

pub static MOCK_PLATFORM_DRIVER: PlatformDriver = PlatformDriver::new(
    "mock_platform_driver",
    OfMatchTable::empty(),
    mock_deferred_probe,
);

pub const MOCK_PLATFORM_DRIVER_REF: DeviceDriverRef = DeviceDriverRef::new(&MOCK_PLATFORM_DRIVER);

fn mock_deferred_probe(
    _device_tree: &DeviceTree,
    _device: DeviceRef,
    _node_id: DeviceNodeId,
) -> ProbeResult {
    ProbeResult::Deferred
}
