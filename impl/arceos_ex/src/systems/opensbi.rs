//! Metadata-only mapping for the runtime `OpenSBI` system.
//!
//! Its formal handoff determines `BootCpuRegisters.a0/a1`, requires entry
//! `satp == 0`, and records ordered-boot/DTB facts. It intentionally does not
//! require `sie/sip == 0`; the kernel's first InterruptType action clears
//! those registers.

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
