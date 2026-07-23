#![allow(dead_code)]

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SpecPath {
    pub layer: &'static str,
    pub path: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MappingStatus {
    MetadataOnly,
    RuntimeImplemented,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SystemMapping {
    pub object_name: &'static str,
    pub charter: SpecPath,
    pub model: SpecPath,
    pub coding: SpecPath,
    pub implementation: &'static str,
    pub status: MappingStatus,
}

pub mod computer;
pub mod kernel;
pub mod opensbi;
pub mod riscv64_platform;
