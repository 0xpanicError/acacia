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
        /// Target function in format ContractName::functionName or ContractName::functionName(args)
        #[arg(value_name = "TARGET")]
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

/// Parsed target with optional signature for overload disambiguation
struct ParsedTarget {
    contract_name: String,
    function_name: String,
    /// Optional signature like "address,uint256" for specific overload
    signature: Option<String>,
}

fn generate_tree(target: &str, output_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Parse target: ContractName::functionName or ContractName::functionName(args)
    let parsed = parse_target(target)?;

    // 1. Discover Foundry project
    let project = FoundryProject::discover()?;
    println!("Found Foundry project at: {:?}", project.root());

    // 2. Find and parse the contract
    let contract_path = project.find_contract(&parsed.contract_name)?;
    println!("Found contract at: {:?}", contract_path);

    // 3. Parse with Solar and extract branch points
    let parser = SolarParser::new(&project);

    match parsed.signature {
        Some(signature) => {
            // Specific signature provided - generate single tree
            println!(
                "Generating BTT tree for {}::{}({})",
                parsed.contract_name, parsed.function_name, signature
            );

            let function_ctx = parser.parse_function_by_signature(
                &contract_path,
                &parsed.contract_name,
                &parsed.function_name,
                &signature,
            )?;

            println!("Found {} branch points", function_ctx.branch_points.len());

            let tree = TreeBuilder::build(&parsed.function_name, function_ctx.branch_points)?;

            // Include signature in filename for overloaded functions
            let output_path = std::path::Path::new(output_dir).join(format!(
                "{}.{}({}).tree",
                parsed.contract_name, parsed.function_name, signature
            ));

            render_tree(&tree, &output_path)?;
            println!("Generated tree at: {:?}", output_path);
        }
        None => {
            // No signature - generate trees for all overloads
            let function_contexts = parser.parse_all_functions(
                &contract_path,
                &parsed.contract_name,
                &parsed.function_name,
            )?;

            let num_overloads = function_contexts.len();
            if num_overloads == 1 {
                // Single function, no overloads - use simple naming
                println!(
                    "Generating BTT tree for {}::{}",
                    parsed.contract_name, parsed.function_name
                );

                let function_ctx = &function_contexts[0];
                println!("Found {} branch points", function_ctx.branch_points.len());

                let tree =
                    TreeBuilder::build(&parsed.function_name, function_ctx.branch_points.clone())?;

                let output_path = std::path::Path::new(output_dir).join(format!(
                    "{}.{}.tree",
                    parsed.contract_name, parsed.function_name
                ));

                render_tree(&tree, &output_path)?;
                println!("Generated tree at: {:?}", output_path);
            } else {
                // Multiple overloads - generate tree for each with signature in filename
                println!(
                    "Found {} overloads for {}::{}",
                    num_overloads, parsed.contract_name, parsed.function_name
                );

                for function_ctx in function_contexts {
                    println!(
                        "Generating tree for {}::{}({}) - {} branch points",
                        parsed.contract_name,
                        parsed.function_name,
                        function_ctx.signature,
                        function_ctx.branch_points.len()
                    );

                    let tree = TreeBuilder::build(
                        &parsed.function_name,
                        function_ctx.branch_points.clone(),
                    )?;

                    let output_path = std::path::Path::new(output_dir).join(format!(
                        "{}.{}({}).tree",
                        parsed.contract_name, parsed.function_name, function_ctx.signature
                    ));

                    render_tree(&tree, &output_path)?;
                    println!("  -> {:?}", output_path);
                }

                println!("Generated {} trees", num_overloads);
            }
        }
    }

    Ok(())
}

fn parse_target(target: &str) -> Result<ParsedTarget, Box<dyn std::error::Error>> {
    let parts: Vec<&str> = target.split("::").collect();
    if parts.len() != 2 {
        return Err(format!(
            "Invalid target format: '{}'. Expected 'ContractName::functionName' or 'ContractName::functionName(args)'",
            target
        )
        .into());
    }

    let contract_name = parts[0].to_string();
    let function_part = parts[1];

    // Check if signature is provided: functionName(args)
    if let Some(open_paren) = function_part.find('(') {
        if function_part.ends_with(')') {
            let function_name = function_part[..open_paren].to_string();
            let signature = function_part[open_paren + 1..function_part.len() - 1].to_string();
            return Ok(ParsedTarget {
                contract_name,
                function_name,
                signature: Some(signature),
            });
        }
    }

    // No signature - just function name
    Ok(ParsedTarget {
        contract_name,
        function_name: function_part.to_string(),
        signature: None,
    })
}
