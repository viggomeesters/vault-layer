use std::env;
use std::path::PathBuf;
use std::process::Command;

use vault_layer_core::{
    cosine_similarity, default_state_dir, deterministic_embedding, embedding_from_json,
    embedding_to_json, scan_vault, sql_literal, write_scan_sqlite, RuntimeConfig, COMMANDS,
    DEFAULT_STATE_SUBDIR,
};

fn main() {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        None | Some("-h") | Some("--help") => print_help(),
        Some("init") => init_command(args.collect()),
        Some("index") => index_command(args.collect()),
        Some("search") => search_command(args.collect()),
        Some("get-note") => get_note_command(args.collect()),
        Some("related") => related_command(args.collect()),
        Some("embed") => embed_command(args.collect()),
        Some("vector-search") => vector_search_command(args.collect()),
        Some("context") => context_command(args.collect()),
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

fn init_command(args: Vec<String>) {
    let vault_path = args.first().cloned().unwrap_or_else(|| "<vault-path>".to_string());
    let state_dir = state_dir_from_args(args).or_else(|| default_state_dir().ok());
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

fn index_command(args: Vec<String>) {
    let vault_path = args.first().cloned().unwrap_or_else(|| "<vault-path>".to_string());
    let state_dir = state_dir_from_args(args);
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

fn search_command(args: Vec<String>) {
    let (query, rest) = split_query(args);
    let options = CliOptions::parse(rest);
    let db = require_db(&options);
    let limit = options.limit.unwrap_or(10);
    let escaped_query = fts_query(&query);
    let like_query = format!("%{}%", query);
    let sql = format!(
        "WITH fts AS (\
         SELECT s.id AS chunk_id, n.path, s.heading_path, snippet(sections_fts, 4, '', '', '…', 16) AS excerpt, -bm25(sections_fts) AS score, s.content_hash, n.modified_unix \
         FROM sections_fts JOIN sections s ON s.id = sections_fts.id JOIN notes n ON n.id = s.note_id \
         WHERE sections_fts MATCH {} LIMIT {}), \
         fallback AS (\
         SELECT s.id AS chunk_id, n.path, s.heading_path, substr(s.text, 1, 240) AS excerpt, 0.0 AS score, s.content_hash, n.modified_unix \
         FROM sections s JOIN notes n ON n.id = s.note_id \
         WHERE s.text LIKE {} OR n.path LIKE {} OR s.heading_path LIKE {} LIMIT {}) \
         SELECT * FROM fts UNION ALL SELECT * FROM fallback WHERE NOT EXISTS (SELECT 1 FROM fts) LIMIT {};",
        sql_literal(&escaped_query), limit, sql_literal(&like_query), sql_literal(&like_query), sql_literal(&like_query), limit, limit
    );
    print_sqlite_json(&db, &sql);
}

fn get_note_command(args: Vec<String>) {
    let (needle, rest) = split_query(args);
    let options = CliOptions::parse(rest);
    let db = require_db(&options);
    let like = format!("%{}%", needle);
    let sql = format!(
        "SELECT n.id, n.path, n.title, n.modified_unix, n.content_hash, substr(group_concat(s.heading_path || ': ' || s.text, char(10)), 1, 4000) AS bounded_content \
         FROM notes n LEFT JOIN sections s ON s.note_id = n.id \
         WHERE n.id = {} OR n.path = {} OR n.path LIKE {} GROUP BY n.id ORDER BY n.path LIMIT 1;",
        sql_literal(&needle), sql_literal(&needle), sql_literal(&like)
    );
    print_sqlite_json(&db, &sql);
}

fn related_command(args: Vec<String>) {
    let (needle, rest) = split_query(args);
    let options = CliOptions::parse(rest);
    let db = require_db(&options);
    let like = format!("%{}%", needle);
    let sql = format!(
        "WITH base AS (SELECT id, path FROM notes WHERE id = {} OR path = {} OR path LIKE {} LIMIT 1), \
         outgoing AS (SELECT l.target AS relation, l.raw AS evidence FROM links l JOIN base b ON b.id = l.source_note_id), \
         incoming AS (SELECT n.path AS relation, l.raw AS evidence FROM links l JOIN notes n ON n.id = l.source_note_id JOIN base b ON l.target = replace(b.path, '.md', '')) \
         SELECT 'outgoing' AS kind, relation, evidence FROM outgoing UNION ALL SELECT 'incoming' AS kind, relation, evidence FROM incoming LIMIT 25;",
        sql_literal(&needle), sql_literal(&needle), sql_literal(&like)
    );
    print_sqlite_json(&db, &sql);
}

fn embed_command(args: Vec<String>) {
    let options = CliOptions::parse(args);
    let db = require_db(&options);
    let rows = sqlite_table(&db, "SELECT id, text FROM sections ORDER BY id;");
    let mut sql = String::from("BEGIN; DELETE FROM embeddings WHERE model = 'deterministic-v0';\n");
    for row in &rows {
        if row.len() < 2 {
            continue;
        }
        let embedding = embedding_to_json(&deterministic_embedding(&row[1], 8));
        sql.push_str(&format!(
            "INSERT OR REPLACE INTO embeddings(chunk_id, model, dimensions, embedding_json, embedded_at_unix) VALUES({}, 'deterministic-v0', 8, {}, strftime('%s','now'));\n",
            sql_literal(&row[0]),
            sql_literal(&embedding)
        ));
    }
    sql.push_str("COMMIT;\n");
    run_sqlite(&db, &sql);
    println!("{{\"model\":\"deterministic-v0\",\"dimensions\":8,\"chunks_embedded\":{}}}", rows.len());
}

fn vector_search_command(args: Vec<String>) {
    let (query, rest) = split_query(args);
    let options = CliOptions::parse(rest);
    let db = require_db(&options);
    let limit = options.limit.unwrap_or(10) as usize;
    let query_embedding = deterministic_embedding(&query, 8);
    let rows = sqlite_table(&db, "SELECT e.chunk_id, n.path, s.heading_path, substr(s.text, 1, 240), s.content_hash, n.modified_unix, e.embedding_json FROM embeddings e JOIN sections s ON s.id = e.chunk_id JOIN notes n ON n.id = s.note_id WHERE e.model = 'deterministic-v0';");
    let mut scored = rows
        .into_iter()
        .filter_map(|row| {
            if row.len() < 7 {
                return None;
            }
            let embedding = embedding_from_json(&row[6]);
            let score = cosine_similarity(&query_embedding, &embedding);
            Some((score, row))
        })
        .collect::<Vec<_>>();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let mut out = String::from("[");
    for (index, (score, row)) in scored.into_iter().take(limit).enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            "{{\"chunk_id\":{},\"path\":{},\"heading_path\":{},\"excerpt\":{},\"score\":{:.6},\"content_hash\":{},\"modified_unix\":{}}}",
            json_string(&row[0]), json_string(&row[1]), json_string(&row[2]), json_string(&row[3]), score, json_string(&row[4]), row[5]
        ));
    }
    out.push(']');
    println!("{out}");
}

fn context_command(args: Vec<String>) {
    let (query, rest) = split_query(args);
    let options = CliOptions::parse(rest);
    let db = require_db(&options);
    let like_query = format!("%{}%", query);
    let sql = format!(
        "SELECT s.id AS chunk_id, n.path, s.heading_path, substr(s.text, 1, 700) AS excerpt, s.content_hash, n.modified_unix \
         FROM sections s JOIN notes n ON n.id = s.note_id \
         WHERE s.text LIKE {} OR n.path LIKE {} OR s.heading_path LIKE {} LIMIT {};",
        sql_literal(&like_query), sql_literal(&like_query), sql_literal(&like_query), options.limit.unwrap_or(8)
    );
    print_sqlite_json(&db, &sql);
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

fn split_query(args: Vec<String>) -> (String, Vec<String>) {
    let mut iter = args.into_iter();
    let query = iter.next().unwrap_or_default();
    (query, iter.collect())
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

fn run_sqlite(db: &PathBuf, sql: &str) {
    let output = Command::new("sqlite3")
        .arg(db)
        .arg(sql)
        .output()
        .unwrap_or_else(|error| fail(&format!("sqlite3 failed to start: {error}")));
    if !output.status.success() {
        fail(&format!("sqlite3 command failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
}

fn sqlite_table(db: &PathBuf, sql: &str) -> Vec<Vec<String>> {
    let output = Command::new("sqlite3")
        .arg("-separator")
        .arg("\t")
        .arg(db)
        .arg(sql)
        .output()
        .unwrap_or_else(|error| fail(&format!("sqlite3 failed to start: {error}")));
    if !output.status.success() {
        fail(&format!("sqlite3 table query failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| line.split('\t').map(ToString::to_string).collect())
        .collect()
}

fn json_string(value: &str) -> String {
    let escaped = value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n");
    format!("\"{escaped}\"")
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
        "VaultLayer\n\nUSAGE:\n    vault-layer <COMMAND> [OPTIONS]\n\nCOMMANDS:\n    init      Initialize config for an external Markdown/Obsidian vault\n    index     Build or refresh the local shadow index outside the repo\n    search    Search indexed vault chunks and return cited JSON results\n    get-note  Return one bounded note with provenance JSON\n    related   Return WikiLink/backlink related notes as JSON\n    embed     Fill deterministic test embeddings for indexed chunks\n    vector-search Search deterministic embeddings with cited JSON results\n    context   Build an agent-ready cited context pack\n    serve     Serve MCP/HTTP interfaces over the local shadow DB\n\nOPTIONS:\n    --state-dir <PATH>    Runtime state directory; default: ~/{DEFAULT_STATE_SUBDIR}\n    --db <PATH>           Shadow DB path for retrieval commands\n    --json                JSON output (retrieval commands already emit JSON)\n\nSAFETY:\n    Vault files are read-only by default. DB/index/vector artifacts must live outside the repo."
    );
}
