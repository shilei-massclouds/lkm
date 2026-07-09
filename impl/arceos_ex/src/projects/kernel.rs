#![allow(dead_code)]

//! Mapping entry for the `KernelProject` engineering product.
//!
//! This module intentionally carries metadata only. Runtime startup order stays
//! in the existing phase/system entry points until a later specification update
//! assigns behavior here.

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
pub struct KernelProjectMapping {
    pub object_name: &'static str,
    pub charter: SpecPath,
    pub model: SpecPath,
    pub coding: SpecPath,
    pub implementation: &'static str,
    pub status: MappingStatus,
}

pub const KERNEL_PROJECT_MAPPING: KernelProjectMapping = KernelProjectMapping {
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
        path: "spec/coding/projects/kernel.spec",
    },
    implementation: "impl/arceos_ex/src/projects/kernel.rs",
    status: MappingStatus::SkeletonOnly,
};
