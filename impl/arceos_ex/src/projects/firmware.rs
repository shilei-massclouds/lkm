//! Metadata mapping for the `FirmwareProject` engineering product.

use super::{MappingStatus, ProjectMapping, SpecPath};

pub const FIRMWARE_PROJECT_MAPPING: ProjectMapping = ProjectMapping {
    object_name: "FirmwareProject",
    charter: SpecPath {
        layer: "charter",
        path: "spec/charter/projects/firmware.md",
    },
    model: SpecPath {
        layer: "model",
        path: "spec/model/projects/firmware.spec",
    },
    coding: SpecPath {
        layer: "coding",
        path: "spec/coding/projects/firmware.md",
    },
    implementation: "impl/arceos_ex/src/projects/firmware.rs",
    status: MappingStatus::SkeletonOnly,
};
