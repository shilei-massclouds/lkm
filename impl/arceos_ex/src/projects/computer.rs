//! Metadata mapping for the `ComputerProject` engineering root.

use super::{MappingStatus, ProjectMapping, SpecPath};

pub const COMPUTER_PROJECT_MAPPING: ProjectMapping = ProjectMapping {
    object_name: "ComputerProject",
    charter: SpecPath {
        layer: "charter",
        path: "spec/charter/projects/computer.md",
    },
    model: SpecPath {
        layer: "model",
        path: "spec/model/projects/computer.spec",
    },
    coding: SpecPath {
        layer: "coding",
        path: "spec/coding/projects/computer.md",
    },
    implementation: "impl/arceos_ex/src/projects/computer.rs",
    status: MappingStatus::SkeletonOnly,
};
