//! Metadata-only mapping for the runtime `Riscv64Platform` system.
//!
//! The platform lifecycle is independent of the BootCPU-owned
//! `BootCpuRegisters`; register updates remain in the real entry path.

use super::{MappingStatus, SpecPath, SystemMapping};

pub const RISCV64_PLATFORM_SYSTEM_MAPPING: SystemMapping = SystemMapping {
    object_name: "Riscv64Platform",
    charter: SpecPath {
        layer: "charter",
        path: "spec/charter/systems/riscv64-platform.md",
    },
    model: SpecPath {
        layer: "model",
        path: "spec/model/systems/riscv64-platform.spec",
    },
    coding: SpecPath {
        layer: "coding",
        path: "spec/coding/systems/riscv64-platform.md",
    },
    implementation: "impl/arceos_ex/src/systems/riscv64_platform.rs",
    status: MappingStatus::MetadataOnly,
};
