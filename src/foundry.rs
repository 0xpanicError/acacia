//! Foundry project discovery and configuration

#![allow(dead_code)]

use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use walkdir::WalkDir;

#[derive(Error, Debug)]
pub enum FoundryError {
    #[error("Could not find foundry.toml in current directory or any parent")]
    ProjectNotFound,

    #[error("Failed to read foundry.toml: {0}")]
    ConfigReadError(#[from] std::io::Error),

    #[error("Failed to parse foundry.toml: {0}")]
    ConfigParseError(#[from] toml::de::Error),

    #[error("Contract '{0}' not found in project")]
    ContractNotFound(String),
}

#[derive(Debug, Deserialize, Default)]
struct FoundryConfig {
    profile: Option<ProfileConfig>,
}

#[derive(Debug, Deserialize, Default)]
struct ProfileConfig {
    default: Option<DefaultProfile>,
}

#[derive(Debug, Deserialize, Default)]
struct DefaultProfile {
    src: Option<String>,
    lib: Option<Vec<String>>,
    remappings: Option<Vec<String>>,
}

#[derive(Debug)]
pub struct FoundryProject {
    pub root: PathBuf,
    pub src_dir: PathBuf,
    pub lib_dirs: Vec<PathBuf>,
    pub remappings: Vec<(String, String)>,
}

impl FoundryProject {
    /// Discover a Foundry project by searching for foundry.toml
    pub fn discover() -> Result<Self, FoundryError> {
        let current_dir = std::env::current_dir()?;
        let root = Self::find_project_root(&current_dir)?;

        // Parse foundry.toml
        let config_path = root.join("foundry.toml");
        let config_content = fs::read_to_string(&config_path)?;
        let config: FoundryConfig = toml::from_str(&config_content)?;

        // Extract configuration with defaults
        let default_profile = config.profile.and_then(|p| p.default).unwrap_or_default();

        let src_dir = root.join(default_profile.src.unwrap_or_else(|| "src".to_string()));

        let lib_dirs: Vec<PathBuf> = default_profile
            .lib
            .unwrap_or_else(|| vec!["lib".to_string()])
            .into_iter()
            .map(|l| root.join(l))
            .collect();

        let remappings = default_profile
            .remappings
            .unwrap_or_default()
            .into_iter()
            .filter_map(|r| {
                let parts: Vec<&str> = r.splitn(2, '=').collect();
                if parts.len() == 2 {
                    Some((parts[0].to_string(), parts[1].to_string()))
                } else {
                    None
                }
            })
            .collect();

        Ok(Self {
            root,
            src_dir,
            lib_dirs,
            remappings,
        })
    }

    fn find_project_root(start: &Path) -> Result<PathBuf, FoundryError> {
        let mut current = start.to_path_buf();

        loop {
            if current.join("foundry.toml").exists() {
                return Ok(current);
            }

            if !current.pop() {
                return Err(FoundryError::ProjectNotFound);
            }
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn src_dir(&self) -> &Path {
        &self.src_dir
    }

    pub fn remappings(&self) -> &[(String, String)] {
        &self.remappings
    }

    /// Find a contract file by contract name
    pub fn find_contract(&self, contract_name: &str) -> Result<PathBuf, FoundryError> {
        // First, look for a file with the exact contract name
        let expected_filename = format!("{}.sol", contract_name);

        // Search in src directory
        for entry in WalkDir::new(&self.src_dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                if let Some(name) = entry.file_name().to_str() {
                    if name == expected_filename {
                        return Ok(entry.path().to_path_buf());
                    }
                }
            }
        }

        // If not found by filename, search file contents for contract definition
        for entry in WalkDir::new(&self.src_dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                if let Some(ext) = entry.path().extension() {
                    if ext == "sol" {
                        if let Ok(content) = fs::read_to_string(entry.path()) {
                            // Simple pattern match for contract definition
                            let pattern = format!("contract {}", contract_name);
                            if content.contains(&pattern) {
                                return Ok(entry.path().to_path_buf());
                            }
                        }
                    }
                }
            }
        }

        Err(FoundryError::ContractNotFound(contract_name.to_string()))
    }

    /// Resolve an import path using remappings
    pub fn resolve_import(&self, import_path: &str, from_file: &Path) -> Option<PathBuf> {
        // Check remappings first
        for (prefix, replacement) in &self.remappings {
            if import_path.starts_with(prefix) {
                let resolved = import_path.replacen(prefix, replacement, 1);
                let path = self.root.join(&resolved);
                if path.exists() {
                    return Some(path);
                }
            }
        }

        // Try relative import
        if import_path.starts_with("./") || import_path.starts_with("../") {
            if let Some(parent) = from_file.parent() {
                let resolved = parent.join(import_path);
                if resolved.exists() {
                    return Some(resolved);
                }
            }
        }

        // Try lib directories
        for lib_dir in &self.lib_dirs {
            let path = lib_dir.join(import_path);
            if path.exists() {
                return Some(path);
            }
        }

        None
    }

    /// Find all Solidity contract files in the src directory
    pub fn find_all_contracts(&self) -> Vec<PathBuf> {
        let mut contracts = Vec::new();

        for entry in WalkDir::new(&self.src_dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                if let Some(ext) = entry.path().extension() {
                    if ext == "sol" {
                        contracts.push(entry.path().to_path_buf());
                    }
                }
            }
        }

        contracts
    }
}
