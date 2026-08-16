//! Machine-readable CLI for Arcana's SQLite data platform.

#[path = "arcana_data/mod.rs"]
mod cli;

fn main() {
    std::process::exit(cli::run());
}
