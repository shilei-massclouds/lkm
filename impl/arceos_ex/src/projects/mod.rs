#![allow(dead_code)]

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
pub struct ProjectMapping {
    pub object_name: &'static str,
    pub charter: SpecPath,
    pub model: SpecPath,
    pub coding: SpecPath,
    pub implementation: &'static str,
    pub status: MappingStatus,
}

pub mod computer;
pub mod firmware;
pub mod hardware;
pub mod kernel;
