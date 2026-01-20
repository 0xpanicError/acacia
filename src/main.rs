mod analysis;
mod cli;
mod foundry;
mod output;
mod parser;
mod tree;

use clap::Parser;
use cli::Cli;

fn main() {
    let cli = Cli::parse();

    if let Err(e) = cli.run() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
