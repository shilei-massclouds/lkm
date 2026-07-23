//! Metadata-only mapping for the runtime-system root `Computer`.

use super::{MappingStatus, SpecPath, SystemMapping};

pub const COMPUTER_SYSTEM_MAPPING: SystemMapping = SystemMapping {
    object_name: "Computer",
    charter: SpecPath {
        layer: "charter",
        path: "spec/charter/systems/computer.md",
    },
    model: SpecPath {
        layer: "model",
        path: "spec/model/systems/computer.spec",
    },
    coding: SpecPath {
        layer: "coding",
        path: "spec/coding/systems/computer.md",
    },
    implementation: "impl/arceos_ex/src/systems/computer.rs",
    status: MappingStatus::MetadataOnly,
};
