use clap::{Parser, Subcommand};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::Path;

use crate::foundry::FoundryProject;
use crate::output::render_to_string;
use crate::parser::SolarParser;
use crate::tree::TreeBuilder;

/// Acacia - BTT Tree Generator for Solidity Smart Contracts
#[derive(Parser)]
#[command(name = "acacia")]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a BTT-style test tree for a function
    Generate {
        /// Target (optional): ContractName, ContractName::functionName, or ContractName::functionName(args)
        /// If omitted, generates trees for all public/external functions in all contracts
        #[arg(value_name = "TARGET", default_value = "")]
        target: String,

        /// Output directory (default: test/trees/)
        #[arg(short, long, default_value = "test/trees")]
        output: String,
    },
}

impl Cli {
    pub fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        match self.command {
            Commands::Generate { target, output } => generate_tree(&target, &output),
        }
    }
}

/// Parsed target with optional contract and function names
enum ParsedTarget {
    /// No target - generate for all contracts in project
    AllContracts,
    /// Contract only - generate for all functions in this contract
    Contract { contract_name: String },
    /// Specific function
    Function {
        contract_name: String,
        function_name: String,
        signature: Option<String>,
    },
}

fn generate_tree(target: &str, output_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    let parsed = parse_target(target);

    // Discover Foundry project
    let project = FoundryProject::discover()?;
    println!("Found Foundry project at: {:?}", project.root());

    let parser = SolarParser::new(&project);

    match parsed {
        // Generate trees for ALL contracts in the project
        ParsedTarget::AllContracts => {
            println!("Generating BTT trees for all contracts in project");

            let contract_files = project.find_all_contracts();
            if contract_files.is_empty() {
                println!("No Solidity files found in src directory");
                return Ok(());
            }

            println!("Found {} Solidity files", contract_files.len());
            let mut total_trees = 0;

            for file_path in contract_files {
                // Get all contracts in this file
                let contracts = match parser.get_contract_names(&file_path) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("Warning: Failed to parse {:?}: {}", file_path, e);
                        continue;
                    }
                };

                for contract_name in contracts {
                    total_trees +=
                        process_contract(&parser, &file_path, &contract_name, output_dir)?;
                }
            }

            println!("Generated {} trees total", total_trees);
        }

        // Generate trees for a specific contract
        ParsedTarget::Contract { contract_name } => {
            let contract_path = project.find_contract(&contract_name)?;
            println!("Found contract at: {:?}", contract_path);

            println!(
                "Generating BTT trees for all public/external functions in {}",
                contract_name
            );

            let count = process_contract(&parser, &contract_path, &contract_name, output_dir)?;

            println!("Generated {} trees for {}", count, contract_name);
        }

        // Generate tree(s) for a specific function
        ParsedTarget::Function {
            contract_name,
            function_name,
            signature,
        } => {
            let contract_path = project.find_contract(&contract_name)?;
            println!("Found contract at: {:?}", contract_path);

            let contract_snake = to_snake_case(&contract_name);
            let contract_output_dir = Path::new(output_dir).join(contract_snake);
            fs::create_dir_all(&contract_output_dir)?;

            match signature {
                Some(sig) => {
                    // Specific signature provided
                    println!(
                        "Generating BTT tree for {}::{}({})",
                        contract_name, function_name, sig
                    );

                    let function_ctx = parser.parse_function_by_signature(
                        &contract_path,
                        &contract_name,
                        &function_name,
                        &sig,
                    )?;

                    println!("Found {} branch points", function_ctx.branch_points.len());

                    let tree = TreeBuilder::build(&function_name, function_ctx.branch_points)?;
                    let content = render_to_string(&tree);

                    // Note: We append only if it exists? Or wait, user wants overloads in same file.
                    // But here we are targeting a specific signature.
                    // If the user specifies a signature, they probably want just that tree.
                    // However, to be consistent with the file naming convention "FunctionName.tree",
                    // if we modify that file, we should probably append to it if it exists, or overwrite?
                    // Given the user said "in case of function overloading ... keep file name the same as function name and add all trees in the same file",
                    // implies if we run this command multiple times for different signatures, they might want them merged.
                    // But for simplicity, if I run for a specific signature, I will write just that tree to "FunctionName.tree".
                    // If they want all, they should run without signature.

                    let output_path = contract_output_dir.join(format!("{}.tree", function_name));

                    let mut file = fs::File::create(&output_path)?;
                    file.write_all(content.as_bytes())?;

                    println!("Generated tree at: {:?}", output_path);
                }
                None => {
                    // No signature - generate for all overloads of this function
                    let function_contexts = parser.parse_all_functions(
                        &contract_path,
                        &contract_name,
                        &function_name,
                    )?;

                    let num_overloads = function_contexts.len();
                    println!(
                        "Found {} overloads for {}::{}",
                        num_overloads, contract_name, function_name
                    );

                    let mut combined_content = String::new();
                    for (i, function_ctx) in function_contexts.iter().enumerate() {
                        let root_name = if num_overloads > 1 {
                            format!("{}({})", function_name, function_ctx.signature)
                        } else {
                            function_name.clone()
                        };

                        let tree =
                            TreeBuilder::build(&root_name, function_ctx.branch_points.clone())?;

                        if i > 0 {
                            combined_content.push_str("\n");
                        }
                        combined_content.push_str(&render_to_string(&tree));
                    }

                    let output_path = contract_output_dir.join(format!("{}.tree", function_name));
                    let mut file = fs::File::create(&output_path)?;
                    file.write_all(combined_content.as_bytes())?;

                    println!("Generated combined tree at: {:?}", output_path);
                }
            }
        }
    }

    Ok(())
}

fn process_contract(
    parser: &SolarParser,
    file_path: &Path,
    contract_name: &str,
    output_dir: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    let function_contexts = parser.parse_all_public_functions(file_path, contract_name)?;

    if function_contexts.is_empty() {
        return Ok(0);
    }

    // Group functions by name
    let mut func_groups: HashMap<String, Vec<&crate::parser::FunctionContext>> = HashMap::new();
    for ctx in &function_contexts {
        func_groups
            .entry(ctx.function_name.clone())
            .or_default()
            .push(ctx);
    }

    let contract_snake = to_snake_case(contract_name);
    let contract_output_dir = Path::new(output_dir).join(contract_snake);
    fs::create_dir_all(&contract_output_dir)?;

    let mut generated_count = 0;

    for (func_name, contexts) in func_groups {
        let mut combined_content = String::new();
        let is_overloaded = contexts.len() > 1;

        // Use the order from contexts, which is deterministic based on parser output (order of definition)
        for (i, ctx) in contexts.iter().enumerate() {
            let root_name = if is_overloaded {
                format!("{}({})", func_name, ctx.signature)
            } else {
                func_name.clone()
            };

            let tree = TreeBuilder::build(&root_name, ctx.branch_points.clone())?;
            if i > 0 {
                combined_content.push_str("\n");
            }
            combined_content.push_str(&render_to_string(&tree));
        }

        let output_path = contract_output_dir.join(format!("{}.tree", func_name));
        let mut file = fs::File::create(&output_path)?;
        file.write_all(combined_content.as_bytes())?;

        println!("  -> {:?}", output_path);
        generated_count += 1;
    }

    Ok(generated_count)
}

fn parse_target(target: &str) -> ParsedTarget {
    // Empty target = all contracts
    if target.is_empty() {
        return ParsedTarget::AllContracts;
    }

    // Check if it contains :: (has function name)
    if let Some(separator_pos) = target.find("::") {
        let contract_name = target[..separator_pos].to_string();
        let function_part = &target[separator_pos + 2..];

        // Check if signature is provided: functionName(args)
        if let Some(open_paren) = function_part.find('(') {
            if function_part.ends_with(')') {
                let function_name = function_part[..open_paren].to_string();
                let signature = function_part[open_paren + 1..function_part.len() - 1].to_string();
                return ParsedTarget::Function {
                    contract_name,
                    function_name,
                    signature: Some(signature),
                };
            }
        }

        // No signature - just function name
        ParsedTarget::Function {
            contract_name,
            function_name: function_part.to_string(),
            signature: None,
        }
    } else {
        // Contract name only
        ParsedTarget::Contract {
            contract_name: target.to_string(),
        }
    }
}

/// Convert CamelCase to snake_case
/// strictness: new word on first uppercase or uppercase preceded by lowercase
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();

    for (i, &c) in chars.iter().enumerate() {
        if c.is_uppercase() {
            // Add underscore if:
            // 1. It's not the first character
            // 2. The previous character is lowercase
            if i > 0 {
                let prev = chars[i - 1];
                if prev.is_lowercase() {
                    result.push('_');
                }
            }
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("Vault"), "vault");
        assert_eq!(to_snake_case("VaultABC"), "vault_abc");
        assert_eq!(to_snake_case("HiHello"), "hi_hello");
        assert_eq!(to_snake_case("myFunction"), "my_function"); // Note: standard conversion, but contract names are usually CapWords
        assert_eq!(to_snake_case("ABC"), "abc");
        assert_eq!(to_snake_case("A"), "a");
    }
}
