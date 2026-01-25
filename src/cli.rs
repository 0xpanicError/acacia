use clap::{Parser, Subcommand};

use crate::foundry::FoundryProject;
use crate::output::render_tree;
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
                    // Get all public/external functions for this contract
                    let function_contexts =
                        match parser.parse_all_public_functions(&file_path, &contract_name) {
                            Ok(f) => f,
                            Err(e) => {
                                eprintln!(
                                    "Warning: Failed to parse functions in {}: {}",
                                    contract_name, e
                                );
                                continue;
                            }
                        };

                    for function_ctx in function_contexts {
                        let func_name = &function_ctx.function_name;
                        let tree =
                            TreeBuilder::build(func_name, function_ctx.branch_points.clone())?;

                        let output_path = std::path::Path::new(output_dir)
                            .join(format!("{}.{}.tree", contract_name, func_name));

                        render_tree(&tree, &output_path)?;
                        println!("  -> {:?}", output_path);
                        total_trees += 1;
                    }
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

            let function_contexts =
                parser.parse_all_public_functions(&contract_path, &contract_name)?;

            if function_contexts.is_empty() {
                println!("No public/external functions found in contract");
                return Ok(());
            }

            let count = function_contexts.len();
            println!("Found {} public/external functions", count);

            for function_ctx in function_contexts {
                let func_name = &function_ctx.function_name;
                println!(
                    "Generating tree for {}::{} - {} branch points",
                    contract_name,
                    func_name,
                    function_ctx.branch_points.len()
                );

                let tree = TreeBuilder::build(func_name, function_ctx.branch_points.clone())?;

                let output_path = std::path::Path::new(output_dir)
                    .join(format!("{}.{}.tree", contract_name, func_name));

                render_tree(&tree, &output_path)?;
                println!("  -> {:?}", output_path);
            }

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

                    let output_path = std::path::Path::new(output_dir)
                        .join(format!("{}.{}({}).tree", contract_name, function_name, sig));

                    render_tree(&tree, &output_path)?;
                    println!("Generated tree at: {:?}", output_path);
                }
                None => {
                    // No signature - generate for all overloads
                    let function_contexts = parser.parse_all_functions(
                        &contract_path,
                        &contract_name,
                        &function_name,
                    )?;

                    let num_overloads = function_contexts.len();
                    if num_overloads == 1 {
                        println!(
                            "Generating BTT tree for {}::{}",
                            contract_name, function_name
                        );

                        let function_ctx = &function_contexts[0];
                        println!("Found {} branch points", function_ctx.branch_points.len());

                        let tree =
                            TreeBuilder::build(&function_name, function_ctx.branch_points.clone())?;

                        let output_path = std::path::Path::new(output_dir)
                            .join(format!("{}.{}.tree", contract_name, function_name));

                        render_tree(&tree, &output_path)?;
                        println!("Generated tree at: {:?}", output_path);
                    } else {
                        println!(
                            "Found {} overloads for {}::{}",
                            num_overloads, contract_name, function_name
                        );

                        for function_ctx in function_contexts {
                            println!(
                                "Generating tree for {}::{}({}) - {} branch points",
                                contract_name,
                                function_name,
                                function_ctx.signature,
                                function_ctx.branch_points.len()
                            );

                            let tree = TreeBuilder::build(
                                &function_name,
                                function_ctx.branch_points.clone(),
                            )?;

                            let output_path = std::path::Path::new(output_dir).join(format!(
                                "{}.{}({}).tree",
                                contract_name, function_name, function_ctx.signature
                            ));

                            render_tree(&tree, &output_path)?;
                            println!("  -> {:?}", output_path);
                        }

                        println!("Generated {} trees", num_overloads);
                    }
                }
            }
        }
    }

    Ok(())
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
