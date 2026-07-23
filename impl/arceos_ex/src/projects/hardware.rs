//! Metadata mapping for the `HardwareProject` engineering product.

use super::{MappingStatus, ProjectMapping, SpecPath};

pub const HARDWARE_PROJECT_MAPPING: ProjectMapping = ProjectMapping {
    object_name: "HardwareProject",
    charter: SpecPath {
        layer: "charter",
        path: "spec/charter/projects/hardware.md",
    },
    model: SpecPath {
        layer: "model",
        path: "spec/model/projects/hardware.spec",
    },
    coding: SpecPath {
        layer: "coding",
        path: "spec/coding/projects/hardware.md",
    },
    implementation: "impl/arceos_ex/src/projects/hardware.rs",
    status: MappingStatus::SkeletonOnly,
};
