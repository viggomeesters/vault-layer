use std::env;
use std::path::PathBuf;
use std::process::Command;

use vault_layer_core::{default_state_dir, scan_vault, sql_literal, write_scan_sqlite, RuntimeConfig, COMMANDS, DEFAULT_STATE_SUBDIR};

fn main() {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        None | Some("-h") | Some("--help") => print_help(),
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
        Some("index") => {
            let vault_path = args.next().unwrap_or_else(|| "<vault-path>".to_string());
            let state_dir = state_dir_from_args(args.collect());
            match RuntimeConfig::new(&vault_path, state_dir) {
                Ok(config) => match scan_vault(&config.vault_path) {
                    Ok(scan) => {
                        let db_path = config.database_path(&scan.vault_id);
                        if let Err(error) = write_scan_sqlite(&scan, &config.vault_path, &db_path) {
                            fail(&format!("index failed: {error}"));
                        }
                        println!("vault-layer index complete");
                        println!("vault_path={vault_path}");
                        println!("read_only=true");
                        println!("notes_indexed={}", scan.notes.len());
                        println!("db_path={}", db_path.display());
                    }
                    Err(error) => fail(&format!("scan failed: {error}")),
                },
                Err(error) => fail(&format!("config failed: {error}")),
            }
        }
        Some("search") => {
            let query = args.next().unwrap_or_default();
            let options = CliOptions::parse(args.collect());
            let db = require_db(&options);
            let limit = options.limit.unwrap_or(10);
            let escaped_query = fts_query(&query);
            let like_query = format!("%{}%", query);
            let sql = format!(
                "WITH fts AS (\
                 SELECT s.id AS chunk_id, n.path, s.heading_path, snippet(sections_fts, 4, '', '', '…', 16) AS excerpt, -bm25(sections_fts) AS score, s.content_hash, n.modified_unix \
                 FROM sections_fts \
                 JOIN sections s ON s.id = sections_fts.id \
                 JOIN notes n ON n.id = s.note_id \
                 WHERE sections_fts MATCH {} \
                 LIMIT {}), \
                 fallback AS (\
                 SELECT s.id AS chunk_id, n.path, s.heading_path, substr(s.text, 1, 240) AS excerpt, 0.0 AS score, s.content_hash, n.modified_unix \
                 FROM sections s JOIN notes n ON n.id = s.note_id \
                 WHERE s.text LIKE {} OR n.path LIKE {} OR s.heading_path LIKE {} \
                 LIMIT {}) \
                 SELECT * FROM fts UNION ALL SELECT * FROM fallback WHERE NOT EXISTS (SELECT 1 FROM fts) LIMIT {};",
                sql_literal(&escaped_query), limit, sql_literal(&like_query), sql_literal(&like_query), sql_literal(&like_query), limit, limit
            );
            print_sqlite_json(&db, &sql);
        }
        Some("get-note") => {
            let needle = args.next().unwrap_or_default();
            let options = CliOptions::parse(args.collect());
            let db = require_db(&options);
            let like = format!("%{}%", needle);
            let sql = format!(
                "SELECT n.id, n.path, n.title, n.modified_unix, n.content_hash, substr(group_concat(s.heading_path || ': ' || s.text, char(10)), 1, 4000) AS bounded_content \
                 FROM notes n LEFT JOIN sections s ON s.note_id = n.id \
                 WHERE n.id = {} OR n.path = {} OR n.path LIKE {} \
                 GROUP BY n.id ORDER BY n.path LIMIT 1;",
                sql_literal(&needle), sql_literal(&needle), sql_literal(&like)
            );
            print_sqlite_json(&db, &sql);
        }
        Some("related") => {
            let needle = args.next().unwrap_or_default();
            let options = CliOptions::parse(args.collect());
            let db = require_db(&options);
            let like = format!("%{}%", needle);
            let sql = format!(
                "WITH base AS (SELECT id, path FROM notes WHERE id = {} OR path = {} OR path LIKE {} LIMIT 1), \
                 outgoing AS (SELECT l.target AS relation, l.raw AS evidence FROM links l JOIN base b ON b.id = l.source_note_id), \
                 incoming AS (SELECT n.path AS relation, l.raw AS evidence FROM links l JOIN notes n ON n.id = l.source_note_id JOIN base b ON l.target = replace(b.path, '.md', '')) \
                 SELECT 'outgoing' AS kind, relation, evidence FROM outgoing \
                 UNION ALL SELECT 'incoming' AS kind, relation, evidence FROM incoming LIMIT 25;",
                sql_literal(&needle), sql_literal(&needle), sql_literal(&like)
            );
            print_sqlite_json(&db, &sql);
        }
        Some("context") => {
            let query = args.next().unwrap_or_default();
            let options = CliOptions::parse(args.collect());
            let db = require_db(&options);
            let like_query = format!("%{}%", query);
            let sql = format!(
                "SELECT s.id AS chunk_id, n.path, s.heading_path, substr(s.text, 1, 700) AS excerpt, s.content_hash, n.modified_unix \
                 FROM sections s JOIN notes n ON n.id = s.note_id \
                 WHERE s.text LIKE {} OR n.path LIKE {} OR s.heading_path LIKE {} \
                 LIMIT {};",
                sql_literal(&like_query), sql_literal(&like_query), sql_literal(&like_query), options.limit.unwrap_or(8)
            );
            print_sqlite_json(&db, &sql);
        }
        Some(command) if COMMANDS.contains(&command) => {
            println!("vault-layer {command}: planned MVP subcommand; implementation follows in child tasks");
        }
        Some(command) => {
            eprintln!("unknown command: {command}\n");
            print_help();
            std::process::exit(2);
        }
    }
}

#[derive(Default)]
struct CliOptions {
    db: Option<PathBuf>,
    limit: Option<u32>,
}

impl CliOptions {
    fn parse(args: Vec<String>) -> Self {
        let mut options = Self::default();
        let mut iter = args.into_iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--db" => options.db = iter.next().map(PathBuf::from),
                "--limit" => options.limit = iter.next().and_then(|value| value.parse().ok()),
                "--json" => {}
                _ if arg.starts_with("--db=") => options.db = Some(PathBuf::from(arg.trim_start_matches("--db="))),
                _ if arg.starts_with("--limit=") => options.limit = arg.trim_start_matches("--limit=").parse().ok(),
                _ => {}
            }
        }
        options
    }
}

fn require_db(options: &CliOptions) -> PathBuf {
    options.db.clone().unwrap_or_else(|| {
        fail("--db <PATH> is required for retrieval commands until config files are implemented")
    })
}

fn fts_query(query: &str) -> String {
    query
        .split_whitespace()
        .map(|part| part.trim_matches(|ch: char| !ch.is_alphanumeric()))
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" AND ")
}

fn print_sqlite_json(db: &PathBuf, sql: &str) {
    let output = Command::new("sqlite3")
        .arg("-json")
        .arg(db)
        .arg(sql)
        .output()
        .unwrap_or_else(|error| fail(&format!("sqlite3 failed to start: {error}")));
    if !output.status.success() {
        fail(&format!("sqlite3 query failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    print!("{}", String::from_utf8_lossy(&output.stdout));
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

fn fail(message: &str) -> ! {
    eprintln!("{message}");
    std::process::exit(1);
}

fn print_help() {
    println!(
        "VaultLayer\n\nUSAGE:\n    vault-layer <COMMAND> [OPTIONS]\n\nCOMMANDS:\n    init      Initialize config for an external Markdown/Obsidian vault\n    index     Build or refresh the local shadow index outside the repo\n    search    Search indexed vault chunks and return cited JSON results\n    get-note  Return one bounded note with provenance JSON\n    related   Return WikiLink/backlink related notes as JSON\n    context   Build an agent-ready cited context pack\n    serve     Serve MCP/HTTP interfaces over the local shadow DB\n\nOPTIONS:\n    --state-dir <PATH>    Runtime state directory; default: ~/{DEFAULT_STATE_SUBDIR}\n    --db <PATH>           Shadow DB path for retrieval commands\n    --json                JSON output (retrieval commands already emit JSON)\n\nSAFETY:\n    Vault files are read-only by default. DB/index/vector artifacts must live outside the repo."
    );
}
