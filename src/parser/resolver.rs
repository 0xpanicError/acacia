//! Import resolution for Solidity files

#![allow(dead_code)]

use crate::foundry::FoundryProject;
use std::path::{Path, PathBuf};

/// Import resolver for handling Solidity imports
pub struct ImportResolver<'a> {
    project: &'a FoundryProject,
}

impl<'a> ImportResolver<'a> {
    pub fn new(project: &'a FoundryProject) -> Self {
        Self { project }
    }

    /// Resolve an import path to an absolute file path
    pub fn resolve(&self, import_path: &str, from_file: &Path) -> Option<PathBuf> {
        self.project.resolve_import(import_path, from_file)
    }
}
