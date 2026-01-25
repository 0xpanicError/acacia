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
        /// Target: ContractName, ContractName::functionName, or ContractName::functionName(args)
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

/// Parsed target with optional function name and signature
struct ParsedTarget {
    contract_name: String,
    /// None = all public/external functions, Some = specific function
    function_name: Option<String>,
    /// Optional signature like "address,uint256" for specific overload
    signature: Option<String>,
}

fn generate_tree(target: &str, output_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    let parsed = parse_target(target)?;

    // 1. Discover Foundry project
    let project = FoundryProject::discover()?;
    println!("Found Foundry project at: {:?}", project.root());

    // 2. Find and parse the contract
    let contract_path = project.find_contract(&parsed.contract_name)?;
    println!("Found contract at: {:?}", contract_path);

    // 3. Parse with Solar and extract branch points
    let parser = SolarParser::new(&project);

    match (&parsed.function_name, &parsed.signature) {
        // Contract only - generate trees for all public/external functions
        (None, _) => {
            println!(
                "Generating BTT trees for all public/external functions in {}",
                parsed.contract_name
            );

            let function_contexts =
                parser.parse_all_public_functions(&contract_path, &parsed.contract_name)?;

            if function_contexts.is_empty() {
                println!("No public/external functions found in contract");
                return Ok(());
            }

            println!(
                "Found {} public/external functions",
                function_contexts.len()
            );

            let count = function_contexts.len();
            for function_ctx in function_contexts {
                let func_name = &function_ctx.function_name;
                println!(
                    "Generating tree for {}::{} - {} branch points",
                    parsed.contract_name,
                    func_name,
                    function_ctx.branch_points.len()
                );

                let tree = TreeBuilder::build(func_name, function_ctx.branch_points.clone())?;

                let output_path = std::path::Path::new(output_dir)
                    .join(format!("{}.{}.tree", parsed.contract_name, func_name));

                render_tree(&tree, &output_path)?;
                println!("  -> {:?}", output_path);
            }

            println!("Generated {} trees for {}", count, parsed.contract_name);
        }

        // Specific function with signature
        (Some(function_name), Some(signature)) => {
            println!(
                "Generating BTT tree for {}::{}({})",
                parsed.contract_name, function_name, signature
            );

            let function_ctx = parser.parse_function_by_signature(
                &contract_path,
                &parsed.contract_name,
                function_name,
                signature,
            )?;

            println!("Found {} branch points", function_ctx.branch_points.len());

            let tree = TreeBuilder::build(function_name, function_ctx.branch_points)?;

            let output_path = std::path::Path::new(output_dir).join(format!(
                "{}.{}({}).tree",
                parsed.contract_name, function_name, signature
            ));

            render_tree(&tree, &output_path)?;
            println!("Generated tree at: {:?}", output_path);
        }

        // Function name without signature - generate for all overloads
        (Some(function_name), None) => {
            let function_contexts =
                parser.parse_all_functions(&contract_path, &parsed.contract_name, function_name)?;

            let num_overloads = function_contexts.len();
            if num_overloads == 1 {
                // Single function, no overloads - use simple naming
                println!(
                    "Generating BTT tree for {}::{}",
                    parsed.contract_name, function_name
                );

                let function_ctx = &function_contexts[0];
                println!("Found {} branch points", function_ctx.branch_points.len());

                let tree = TreeBuilder::build(function_name, function_ctx.branch_points.clone())?;

                let output_path = std::path::Path::new(output_dir)
                    .join(format!("{}.{}.tree", parsed.contract_name, function_name));

                render_tree(&tree, &output_path)?;
                println!("Generated tree at: {:?}", output_path);
            } else {
                // Multiple overloads - generate tree for each with signature in filename
                println!(
                    "Found {} overloads for {}::{}",
                    num_overloads, parsed.contract_name, function_name
                );

                for function_ctx in function_contexts {
                    println!(
                        "Generating tree for {}::{}({}) - {} branch points",
                        parsed.contract_name,
                        function_name,
                        function_ctx.signature,
                        function_ctx.branch_points.len()
                    );

                    let tree =
                        TreeBuilder::build(function_name, function_ctx.branch_points.clone())?;

                    let output_path = std::path::Path::new(output_dir).join(format!(
                        "{}.{}({}).tree",
                        parsed.contract_name, function_name, function_ctx.signature
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
    // Check if it contains :: (has function name)
    if let Some(separator_pos) = target.find("::") {
        let contract_name = target[..separator_pos].to_string();
        let function_part = &target[separator_pos + 2..];

        // Check if signature is provided: functionName(args)
        if let Some(open_paren) = function_part.find('(') {
            if function_part.ends_with(')') {
                let function_name = function_part[..open_paren].to_string();
                let signature = function_part[open_paren + 1..function_part.len() - 1].to_string();
                return Ok(ParsedTarget {
                    contract_name,
                    function_name: Some(function_name),
                    signature: Some(signature),
                });
            }
        }

        // No signature - just function name
        Ok(ParsedTarget {
            contract_name,
            function_name: Some(function_part.to_string()),
            signature: None,
        })
    } else {
        // Contract name only - generate for all public/external functions
        Ok(ParsedTarget {
            contract_name: target.to_string(),
            function_name: None,
            signature: None,
        })
    }
}
