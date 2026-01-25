//! Import and inheritance resolution for Solidity files

#![allow(dead_code)]

use crate::foundry::FoundryProject;
use solar_parse::ast::{self, ItemKind};
use solar_parse::interface::Session;
use solar_parse::Parser;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Modifier definition extracted from a contract
#[derive(Debug, Clone)]
pub struct ModifierDef {
    pub name: String,
    pub has_body: bool,
}

/// Parsed contract info
#[derive(Debug)]
struct ParsedContract {
    name: String,
    parents: Vec<String>,
    modifiers: Vec<String>,
}

/// Parsed file info
#[derive(Debug)]
struct ParsedFile {
    path: PathBuf,
    imports: Vec<String>,
    contracts: Vec<ParsedContract>,
}

/// Import and inheritance resolver
pub struct InheritanceResolver<'a> {
    project: &'a FoundryProject,
    file_cache: HashMap<PathBuf, ParsedFile>,
}

impl<'a> InheritanceResolver<'a> {
    pub fn new(project: &'a FoundryProject) -> Self {
        Self {
            project,
            file_cache: HashMap::new(),
        }
    }

    /// Resolve an import path to an absolute file path
    pub fn resolve_import(&self, import_path: &str, from_file: &Path) -> Option<PathBuf> {
        self.project.resolve_import(import_path, from_file)
    }

    /// Parse a file and cache the results
    fn parse_and_cache(&mut self, file_path: &Path) -> Option<&ParsedFile> {
        if self.file_cache.contains_key(file_path) {
            return self.file_cache.get(file_path);
        }

        let sess = Session::builder().with_silent_emitter(None).build();

        let parsed = sess.enter(|| {
            let arena = ast::Arena::new();
            let mut parser = Parser::from_file(&sess, &arena, file_path).ok()?;

            let source_unit = parser.parse_file().ok()?;

            // Extract imports
            let mut imports = Vec::new();
            for item in source_unit.items.iter() {
                if let ItemKind::Import(import) = &item.kind {
                    // Get the import path as a string
                    let path_str = import.path.value.as_str();
                    imports.push(path_str.to_string());
                }
            }

            // Extract contracts with their inheritance
            let mut contracts = Vec::new();
            for item in source_unit.items.iter() {
                if let ItemKind::Contract(contract) = &item.kind {
                    let name = contract.name.to_string();

                    // Get parent contract names
                    let parents: Vec<String> = contract
                        .bases
                        .iter()
                        .map(|base| base.name.last().to_string())
                        .collect();

                    // Get modifier names
                    let modifiers: Vec<String> = contract
                        .body
                        .iter()
                        .filter_map(|item| {
                            if let ItemKind::Function(func) = &item.kind {
                                if func.kind == ast::FunctionKind::Modifier {
                                    return func.header.name.as_ref().map(|n| n.to_string());
                                }
                            }
                            None
                        })
                        .collect();

                    contracts.push(ParsedContract {
                        name,
                        parents,
                        modifiers,
                    });
                }
            }

            Some(ParsedFile {
                path: file_path.to_path_buf(),
                imports,
                contracts,
            })
        });

        if let Some(parsed) = parsed {
            self.file_cache.insert(file_path.to_path_buf(), parsed);
            self.file_cache.get(file_path)
        } else {
            None
        }
    }

    /// Find which file contains a contract by searching imports
    pub fn find_contract_file(&mut self, contract_name: &str, from_file: &Path) -> Option<PathBuf> {
        // First parse the current file
        let parsed = self.parse_and_cache(from_file)?;

        // Check if contract is in current file
        if parsed.contracts.iter().any(|c| c.name == contract_name) {
            return Some(from_file.to_path_buf());
        }

        // Search in imports
        let imports: Vec<String> = parsed.imports.clone();
        for import_path in imports {
            if let Some(resolved) = self.resolve_import(&import_path, from_file) {
                if let Some(parsed_import) = self.parse_and_cache(&resolved) {
                    if parsed_import
                        .contracts
                        .iter()
                        .any(|c| c.name == contract_name)
                    {
                        return Some(resolved);
                    }
                }
            }
        }

        None
    }

    /// Get parent contract names for a contract
    pub fn get_parent_names(&mut self, contract_name: &str, file_path: &Path) -> Vec<String> {
        if let Some(parsed) = self.parse_and_cache(file_path) {
            if let Some(contract) = parsed.contracts.iter().find(|c| c.name == contract_name) {
                return contract.parents.clone();
            }
        }
        Vec::new()
    }

    /// Get all modifier names from a contract (not including inherited)
    pub fn get_modifier_names(&mut self, contract_name: &str, file_path: &Path) -> Vec<String> {
        if let Some(parsed) = self.parse_and_cache(file_path) {
            if let Some(contract) = parsed.contracts.iter().find(|c| c.name == contract_name) {
                return contract.modifiers.clone();
            }
        }
        Vec::new()
    }

    /// Build the full inheritance chain for a contract
    /// Returns list of (file_path, contract_name) from root ancestor to child
    pub fn build_inheritance_chain(
        &mut self,
        contract_name: &str,
        file_path: &Path,
    ) -> Vec<(PathBuf, String)> {
        let mut chain = Vec::new();
        let mut visited = std::collections::HashSet::new();

        self.build_chain_recursive(contract_name, file_path, &mut chain, &mut visited);

        chain
    }

    fn build_chain_recursive(
        &mut self,
        contract_name: &str,
        file_path: &Path,
        chain: &mut Vec<(PathBuf, String)>,
        visited: &mut std::collections::HashSet<String>,
    ) {
        if visited.contains(contract_name) {
            return; // Avoid cycles
        }
        visited.insert(contract_name.to_string());

        // Get parents
        let parents = self.get_parent_names(contract_name, file_path);

        // Recurse into parents first (so they appear before children in the chain)
        for parent_name in parents {
            if let Some(parent_file) = self.find_contract_file(&parent_name, file_path) {
                self.build_chain_recursive(&parent_name, &parent_file, chain, visited);
            }
        }

        // Add current contract
        chain.push((file_path.to_path_buf(), contract_name.to_string()));
    }
}
