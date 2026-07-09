#![allow(dead_code)]

//! Mapping entry for the running `Kernel` system instance.
//!
//! This module intentionally carries metadata only. The current
//! `startup_timeline_ready` and `startup_timeline_event` functions remain in
//! the crate root as the compatibility boundary for existing startup behavior.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SpecPath {
    pub layer: &'static str,
    pub path: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MappingStatus {
    SkeletonOnly,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KernelSystemMapping {
    pub object_name: &'static str,
    pub charter: SpecPath,
    pub model: SpecPath,
    pub coding: SpecPath,
    pub implementation: &'static str,
    pub status: MappingStatus,
}

pub const KERNEL_SYSTEM_MAPPING: KernelSystemMapping = KernelSystemMapping {
    object_name: "Kernel",
    charter: SpecPath {
        layer: "charter",
        path: "spec/charter/systems/kernel.md",
    },
    model: SpecPath {
        layer: "model",
        path: "spec/model/systems/kernel.spec",
    },
    coding: SpecPath {
        layer: "coding",
        path: "spec/coding/systems/kernel.spec",
    },
    implementation: "impl/arceos_ex/src/systems/kernel.rs",
    status: MappingStatus::SkeletonOnly,
};
