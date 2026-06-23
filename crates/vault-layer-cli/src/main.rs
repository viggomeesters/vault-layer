use std::env;
use std::path::PathBuf;

use vault_layer_core::{default_state_dir, COMMANDS, DEFAULT_STATE_SUBDIR};

fn main() {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        None | Some("-h") | Some("--help") => {
            print_help();
        }
        Some("init") => {
            let vault_path = args.next().unwrap_or_else(|| "<vault-path>".to_string());
            let state_dir = state_dir_from_args(args.collect());
            let state_dir = state_dir.or_else(|| default_state_dir().ok());
            println!("VaultLayer init plan");
            println!("vault_path={vault_path}");
            println!(
                "state_dir={}",
                state_dir
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| format!("~/{DEFAULT_STATE_SUBDIR}"))
            );
            println!("writeback=disabled");
        }
        Some(command) if COMMANDS.contains(&command) => {
            println!("vault-layer {command}: planned MVP subcommand; implementation follows in child tasks");
        }
        Some(command) => {
            eprintln!("unknown command: {command}
");
            print_help();
            std::process::exit(2);
        }
    }
}

fn state_dir_from_args(args: Vec<String>) -> Option<PathBuf> {
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        if arg == "--state-dir" {
            return iter.next().map(PathBuf::from);
        }
        if let Some(value) = arg.strip_prefix("--state-dir=") {
            return Some(PathBuf::from(value));
        }
    }
    None
}

fn print_help() {
    println!(
        "VaultLayer

USAGE:
    vault-layer <COMMAND> [OPTIONS]

COMMANDS:
    init      Initialize config for an external Markdown/Obsidian vault
    index     Build or refresh the local shadow index outside the repo
    search    Search indexed vault chunks and return cited results
    context   Build an agent-ready cited context pack
    serve     Serve MCP/HTTP interfaces over the local shadow DB

OPTIONS:
    --state-dir <PATH>    Runtime state directory; default: ~/{DEFAULT_STATE_SUBDIR}

SAFETY:
    Vault files are read-only by default. DB/index/vector artifacts must live outside the repo."
    );
}
