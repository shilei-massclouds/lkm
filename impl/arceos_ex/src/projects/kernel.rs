#![allow(dead_code)]

//! Mapping entry for the `KernelProject` engineering product.
//!
//! This module intentionally carries metadata only. Runtime startup order stays
//! in the existing phase/system entry points until a later specification update
//! assigns behavior here.

use super::{MappingStatus, ProjectMapping, SpecPath};

pub const KERNEL_PROJECT_MAPPING: ProjectMapping = ProjectMapping {
    object_name: "KernelProject",
    charter: SpecPath {
        layer: "charter",
        path: "spec/charter/projects/kernel.md",
    },
    model: SpecPath {
        layer: "model",
        path: "spec/model/projects/kernel.spec",
    },
    coding: SpecPath {
        layer: "coding",
        path: "spec/coding/projects/kernel.md",
    },
    implementation: "impl/arceos_ex/src/projects/kernel.rs",
    status: MappingStatus::SkeletonOnly,
};
