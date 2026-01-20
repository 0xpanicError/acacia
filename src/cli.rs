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
        /// Target function in format ContractName::functionName
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

fn generate_tree(target: &str, output_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Parse target: ContractName::functionName
    let (contract_name, function_name) = parse_target(target)?;

    println!(
        "Generating BTT tree for {}::{}",
        contract_name, function_name
    );

    // 1. Discover Foundry project
    let project = FoundryProject::discover()?;
    println!("Found Foundry project at: {:?}", project.root());

    // 2. Find and parse the contract
    let contract_path = project.find_contract(&contract_name)?;
    println!("Found contract at: {:?}", contract_path);

    // 3. Parse with Solar and extract branch points
    let parser = SolarParser::new(&project);
    let function_ctx = parser.parse_function(&contract_path, &contract_name, &function_name)?;

    println!("Found {} branch points", function_ctx.branch_points.len());

    // 4. Build the tree
    let tree = TreeBuilder::build(&function_name, function_ctx.branch_points)?;

    // 5. Render and write output
    let output_path =
        std::path::Path::new(output_dir).join(format!("{}.{}.tree", contract_name, function_name));

    render_tree(&tree, &output_path)?;

    println!("Generated tree at: {:?}", output_path);

    Ok(())
}

fn parse_target(target: &str) -> Result<(String, String), Box<dyn std::error::Error>> {
    let parts: Vec<&str> = target.split("::").collect();
    if parts.len() != 2 {
        return Err(format!(
            "Invalid target format: '{}'. Expected 'ContractName::functionName'",
            target
        )
        .into());
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}
