//! Metadata-only mapping for the runtime `OpenSBI` system.
//!
//! Its handoff determines only `BootCpuRegisters.a0/a1`; the remaining entry
//! registers are updated by the kernel's existing assembly path.

use super::{MappingStatus, SpecPath, SystemMapping};

pub const OPENSBI_SYSTEM_MAPPING: SystemMapping = SystemMapping {
    object_name: "OpenSBI",
    charter: SpecPath {
        layer: "charter",
        path: "spec/charter/systems/opensbi.md",
    },
    model: SpecPath {
        layer: "model",
        path: "spec/model/systems/opensbi.spec",
    },
    coding: SpecPath {
        layer: "coding",
        path: "spec/coding/systems/opensbi.md",
    },
    implementation: "impl/arceos_ex/src/systems/opensbi.rs",
    status: MappingStatus::MetadataOnly,
};
