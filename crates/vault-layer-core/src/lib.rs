//! Core path, scanner, and runtime primitives for VaultLayer.
//!
//! The vault remains the source of truth. Runtime indexes, databases, caches,
//! and embeddings live outside the repository and outside the vault by default.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// Default runtime directory relative to the user's home directory.
pub const DEFAULT_STATE_SUBDIR: &str = ".local/share/vault-layer";

/// Commands planned for the first public CLI surface.
pub const COMMANDS: &[&str] = &[
    "init",
    "index",
    "search",
    "context",
    "serve",
    "backend-info",
    "sqlite-vec-info",
    "sync-turso",
];

/// Supported storage backends. Local SQLite + FTS5 is the primary retrieval default.
/// DuckDB is an explicit analytics/export sidecar. Local libSQL is the local open-source Turso-compatible engine and needs no URL/token.
/// Turso/libSQL remote is configured explicitly and never guessed from repo state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageBackendKind {
    LocalDuckdb,
    LocalSqlite,
    LocalLibsql,
    TursoRemote,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageBackendConfig {
    pub kind: StorageBackendKind,
    pub database_url: Option<String>,
    pub auth_token_present: bool,
}

impl StorageBackendConfig {
    pub fn local_duckdb() -> Self {
        Self {
            kind: StorageBackendKind::LocalDuckdb,
            database_url: None,
            auth_token_present: false,
        }
    }

    pub fn local_sqlite() -> Self {
        Self {
            kind: StorageBackendKind::LocalSqlite,
            database_url: None,
            auth_token_present: false,
        }
    }

    pub fn local_libsql() -> Self {
        Self {
            kind: StorageBackendKind::LocalLibsql,
            database_url: None,
            auth_token_present: false,
        }
    }

    pub fn from_env() -> Self {
        let backend = env::var("VAULT_LAYER_BACKEND")
            .ok()
            .map(|value| value.trim().to_ascii_lowercase())
            .unwrap_or_default();
        match backend.as_str() {
            "duckdb" | "duckdb-local" => Self::local_duckdb(),
            "sqlite" | "sqlite-local" => Self::local_sqlite(),
            "libsql" | "libsql-local" | "turso-local" => Self::local_libsql(),
            "turso" | "turso-remote" | "libsql-remote" => match env::var("TURSO_DATABASE_URL")
                .ok()
                .filter(|value| !value.trim().is_empty())
            {
                Some(url) => Self {
                    kind: StorageBackendKind::TursoRemote,
                    database_url: Some(url),
                    auth_token_present: env::var("TURSO_AUTH_TOKEN")
                        .is_ok_and(|value| !value.trim().is_empty()),
                },
                None => Self {
                    kind: StorageBackendKind::TursoRemote,
                    database_url: None,
                    auth_token_present: false,
                },
            },
            _ => match env::var("TURSO_DATABASE_URL")
                .ok()
                .filter(|value| !value.trim().is_empty())
            {
                Some(url) => Self {
                    kind: StorageBackendKind::TursoRemote,
                    database_url: Some(url),
                    auth_token_present: env::var("TURSO_AUTH_TOKEN")
                        .is_ok_and(|value| !value.trim().is_empty()),
                },
                None => Self::local_sqlite(),
            },
        }
    }

    pub fn backend_name(&self) -> &'static str {
        match self.kind {
            StorageBackendKind::LocalDuckdb => "duckdb",
            StorageBackendKind::LocalSqlite => "sqlite",
            StorageBackendKind::LocalLibsql => "libsql-local",
            StorageBackendKind::TursoRemote => "turso-libsql",
        }
    }

    pub fn index_write_mode(&self) -> &'static str {
        match self.kind {
            StorageBackendKind::LocalDuckdb => "implemented-analytics-sidecar",
            StorageBackendKind::LocalSqlite => "implemented-primary-local-retrieval",
            StorageBackendKind::LocalLibsql => "implemented-local-open-source-libsql",
            StorageBackendKind::TursoRemote => "implemented-explicit-remote-sync",
        }
    }

    pub fn vector_mode(&self) -> &'static str {
        match self.kind {
            StorageBackendKind::LocalDuckdb => "duckdb-analytics-portable-json-cosine",
            StorageBackendKind::LocalSqlite => "sqlite-vec-target-json-cosine-fallback",
            StorageBackendKind::LocalLibsql => "portable-json-cosine-on-libsql",
            StorageBackendKind::TursoRemote => "native-libsql-vector-target",
        }
    }
}

/// SQL fragment documenting the native libSQL/Turso vector target shape.
/// Local SQLite keeps JSON vectors so smoke tests never require remote credentials.
pub const LIBSQL_VECTOR_TARGET_SQL: &str =
    "embedding F32_BLOB(1536); CREATE INDEX chunk_embedding_idx ON embeddings (libsql_vector_idx(embedding, 'metric=cosine'));";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeConfig {
    pub vault_path: PathBuf,
    pub state_dir: PathBuf,
}

impl RuntimeConfig {
    pub fn new(vault_path: impl Into<PathBuf>, state_dir: Option<PathBuf>) -> Result<Self, String> {
        let vault_path = vault_path.into();
        if vault_path.as_os_str().is_empty() {
            return Err("vault path cannot be empty".to_string());
        }
        let state_dir = match state_dir {
            Some(path) => path,
            None => default_state_dir()?,
        };
        if is_inside(&state_dir, &vault_path) {
            return Err(format!(
                "state directory must be outside the vault: {} is inside {}",
                state_dir.display(),
                vault_path.display()
            ));
        }
        Ok(Self {
            vault_path,
            state_dir,
        })
    }

    pub fn database_path(&self, vault_id: &str) -> PathBuf {
        self.state_dir.join(vault_id).join("vault-layer.db")
    }

    pub fn duckdb_database_path(&self, vault_id: &str) -> PathBuf {
        self.state_dir.join(vault_id).join("vault-layer.duckdb")
    }

    pub fn libsql_database_path(&self, vault_id: &str) -> PathBuf {
        self.state_dir.join(vault_id).join("vault-layer.libsql")
    }
}

pub fn default_state_dir() -> Result<PathBuf, String> {
    match env::var_os("VAULT_LAYER_STATE_DIR") {
        Some(value) if !value.is_empty() => Ok(PathBuf::from(value)),
        _ => {
            let home = env::var_os("HOME")
                .ok_or_else(|| "HOME is not set; pass --state-dir".to_string())?;
            Ok(PathBuf::from(home).join(DEFAULT_STATE_SUBDIR))
        }
    }
}

pub fn is_inside(child: &Path, parent: &Path) -> bool {
    let child_components: Vec<_> = child.components().collect();
    let parent_components: Vec<_> = parent.components().collect();
    child_components.starts_with(&parent_components)
}

#[derive(Debug, Clone, PartialEq)]
pub struct NoteRecord {
    pub id: String,
    pub path: String,
    pub title: String,
    pub modified_unix: u64,
    pub content_hash: String,
    pub human_relevance_score: f32,
    pub frontmatter: Vec<(String, String)>,
    pub sections: Vec<SectionRecord>,
    pub links: Vec<LinkRecord>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SectionRecord {
    pub id: String,
    pub note_id: String,
    pub heading_path: String,
    pub level: u8,
    pub text: String,
    pub content_hash: String,
    pub human_relevance_score: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkRecord {
    pub source_note_id: String,
    pub target: String,
    pub raw: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VaultScan {
    pub vault_id: String,
    pub notes: Vec<NoteRecord>,
}

pub fn scan_vault(vault_path: &Path) -> Result<VaultScan, String> {
    scan_vault_limited(vault_path, None)
}

pub fn scan_vault_limited(vault_path: &Path, limit: Option<usize>) -> Result<VaultScan, String> {
    let mut files = Vec::new();
    collect_markdown_files(vault_path, vault_path, &mut files, limit)?;
    files.sort();
    if let Some(limit) = limit {
        files.truncate(limit);
    }
    let vault_id = stable_id("vault", &vault_path.to_string_lossy());
    let mut notes = Vec::new();
    for relative in files {
        let absolute = vault_path.join(&relative);
        let content = fs::read_to_string(&absolute)
            .map_err(|err| format!("read {}: {err}", absolute.display()))?;
        let metadata = fs::metadata(&absolute)
            .map_err(|err| format!("metadata {}: {err}", absolute.display()))?;
        notes.push(parse_note(&vault_id, &relative, &content, &metadata)?);
    }
    Ok(VaultScan { vault_id, notes })
}

fn collect_markdown_files(
    root: &Path,
    current: &Path,
    out: &mut Vec<String>,
    limit: Option<usize>,
) -> Result<(), String> {
    if limit.is_some_and(|max| out.len() >= max) {
        return Ok(());
    }
    let mut entries = fs::read_dir(current)
        .map_err(|err| format!("read_dir {}: {err}", current.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("dir entry: {err}"))?;
    entries.sort_by_key(|entry| entry.path());
    for entry in entries {
        if limit.is_some_and(|max| out.len() >= max) {
            break;
        }
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') || name == "node_modules" {
            continue;
        }
        if path.is_dir() {
            collect_markdown_files(root, &path, out, limit)?;
        } else if path.extension().is_some_and(|ext| ext == "md") {
            let relative = path.strip_prefix(root).map_err(|err| err.to_string())?;
            out.push(relative.to_string_lossy().replace('\\', "/"));
        }
    }
    Ok(())
}

pub fn parse_note(
    vault_id: &str,
    relative_path: &str,
    content: &str,
    metadata: &fs::Metadata,
) -> Result<NoteRecord, String> {
    let content_hash = stable_hash(content);
    let note_id = stable_id(
        "note",
        &format!("{vault_id}:{relative_path}:{content_hash}"),
    );
    let modified_unix = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    let (frontmatter, body) = parse_frontmatter(content);
    let title = frontmatter
        .iter()
        .find(|(key, _)| key == "title")
        .map(|(_, value)| value.clone())
        .unwrap_or_else(|| title_from_path(relative_path));
    let human_relevance_score = human_relevance_score(&frontmatter, relative_path);
    let sections = parse_sections(&note_id, body, human_relevance_score);
    let links = extract_wikilinks(&note_id, content);
    let tags = extract_tags(content);
    Ok(NoteRecord {
        id: note_id,
        path: relative_path.to_string(),
        title,
        modified_unix,
        content_hash,
        human_relevance_score,
        frontmatter,
        sections,
        links,
        tags,
    })
}

fn parse_frontmatter(content: &str) -> (Vec<(String, String)>, &str) {
    if !content.starts_with(
        "---
",
    ) {
        return (Vec::new(), content);
    }
    let rest = &content[4..];
    if let Some(end) = rest.find(
        "
---
",
    ) {
        let raw = &rest[..end];
        let body = &rest[end + 5..];
        let pairs = raw
            .lines()
            .filter_map(|line| line.split_once(':'))
            .map(|(key, value)| {
                (
                    key.trim().to_string(),
                    value.trim().trim_matches('"').to_string(),
                )
            })
            .collect();
        (pairs, body)
    } else {
        (Vec::new(), content)
    }
}

fn parse_sections(note_id: &str, body: &str, human_relevance_score: f32) -> Vec<SectionRecord> {
    let mut sections = Vec::new();
    let mut section_index = 0usize;
    let mut current_heading = String::from("root");
    let mut current_level = 0_u8;
    let mut buffer = String::new();
    for line in body.lines() {
        if let Some((level, heading)) = parse_heading(line) {
            push_section(
                &mut sections,
                note_id,
                &current_heading,
                current_level,
                &buffer,
                human_relevance_score,
                section_index,
            );
            section_index += 1;
            current_heading = heading;
            current_level = level;
            buffer.clear();
        } else {
            buffer.push_str(line);
            buffer.push('\n');
        }
    }
    push_section(
        &mut sections,
        note_id,
        &current_heading,
        current_level,
        &buffer,
        human_relevance_score,
        section_index,
    );
    sections
}

fn parse_heading(line: &str) -> Option<(u8, String)> {
    let trimmed = line.trim_start();
    let hashes = trimmed.chars().take_while(|ch| *ch == '#').count();
    if hashes == 0 || hashes > 6 || !trimmed.chars().nth(hashes).is_some_and(|ch| ch == ' ') {
        return None;
    }
    Some((hashes as u8, trimmed[hashes + 1..].trim().to_string()))
}

fn push_section(
    sections: &mut Vec<SectionRecord>,
    note_id: &str,
    heading: &str,
    level: u8,
    text: &str,
    human_relevance_score: f32,
    section_index: usize,
) {
    let trimmed = text.trim();
    if trimmed.is_empty() && heading == "root" {
        return;
    }
    let content_hash = stable_hash(trimmed);
    let id = stable_id(
        "chunk",
        &format!("{note_id}:{heading}:{section_index}:{content_hash}"),
    );
    sections.push(SectionRecord {
        id,
        note_id: note_id.to_string(),
        heading_path: heading.to_string(),
        level,
        text: trimmed.to_string(),
        content_hash,
        human_relevance_score,
    });
}

fn extract_wikilinks(note_id: &str, content: &str) -> Vec<LinkRecord> {
    let mut links = Vec::new();
    let mut rest = content;
    while let Some(start) = rest.find("[[") {
        rest = &rest[start + 2..];
        let Some(end) = rest.find("]]") else {
            break;
        };
        let raw_target = &rest[..end];
        let target = raw_target
            .split('|')
            .next()
            .unwrap_or(raw_target)
            .trim()
            .to_string();
        links.push(LinkRecord {
            source_note_id: note_id.to_string(),
            target,
            raw: format!("[[{raw_target}]]"),
        });
        rest = &rest[end + 2..];
    }
    links
}

fn extract_tags(content: &str) -> Vec<String> {
    let mut tags = Vec::new();
    for token in content.split_whitespace() {
        let token = token
            .trim_matches(|ch: char| ch == ',' || ch == '.' || ch == ';' || ch == ')' || ch == '(');
        if let Some(tag) = token.strip_prefix('#') {
            if !tag.is_empty()
                && tag
                    .chars()
                    .all(|ch| ch.is_alphanumeric() || ch == '-' || ch == '_' || ch == '/')
            {
                let tag = tag.to_string();
                if !tags.contains(&tag) {
                    tags.push(tag);
                }
            }
        }
    }
    tags
}

fn human_relevance_score(frontmatter: &[(String, String)], relative_path: &str) -> f32 {
    for key in ["human_relevance_score", "human_relevance", "human_score"] {
        if let Some((_, value)) = frontmatter.iter().find(|(candidate, _)| candidate == key) {
            if let Ok(score) = value.parse::<f32>() {
                return score.clamp(0.0, 1.0);
            }
        }
    }
    if frontmatter
        .iter()
        .any(|(key, value)| key == "audience" && value.eq_ignore_ascii_case("system"))
        || frontmatter.iter().any(|(key, value)| {
            key == "system_only" && matches!(value.as_str(), "true" | "yes" | "1")
        })
    {
        return 0.1;
    }
    if frontmatter
        .iter()
        .any(|(key, value)| key == "audience" && value.eq_ignore_ascii_case("human"))
    {
        return 0.9;
    }
    if relative_path.starts_with("system/") || relative_path.contains("/system/") {
        return 0.25;
    }
    0.5
}

fn title_from_path(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(path)
        .replace('-', " ")
}

pub fn stable_id(prefix: &str, input: &str) -> String {
    format!("{prefix}_{}", stable_hash(input))
}

pub fn stable_hash(input: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

/// SQL schema used for local SQLite/libSQL-compatible shadow databases.
pub const SQLITE_SCHEMA: &str = include_str!("schema.sql");

/// Write a complete scan into a local SQLite database by invoking `sqlite3`.
///
/// This keeps the first MVP dependency-light while preserving a real DB file.
pub fn write_scan_sqlite(
    scan: &VaultScan,
    vault_root: &Path,
    db_path: &Path,
) -> Result<(), String> {
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("create state dir {}: {err}", parent.display()))?;
    }
    if db_path.exists() {
        fs::remove_file(db_path)
            .map_err(|err| format!("replace existing shadow db {}: {err}", db_path.display()))?;
    }
    let mut child = std::process::Command::new("sqlite3")
        .arg(db_path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|err| format!("spawn sqlite3: {err}"))?;
    {
        use std::io::Write;
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| "sqlite3 stdin unavailable".to_string())?;
        stdin
            .write_all(SQLITE_SCHEMA.as_bytes())
            .map_err(|err| format!("write sqlite schema: {err}"))?;
        stdin
            .write_all(b"\nBEGIN;\nDELETE FROM sections_fts;\nDELETE FROM vaults;\n")
            .map_err(|err| format!("write sqlite prelude: {err}"))?;
        writeln!(
            stdin,
            "INSERT INTO vaults(id, root_path, indexed_at_unix) VALUES({}, {}, strftime('%s','now'));",
            sql_quote(&scan.vault_id),
            sql_quote(&vault_root.to_string_lossy())
        )
        .map_err(|err| format!("write vault row: {err}"))?;
        for note in &scan.notes {
            writeln!(
                stdin,
                "INSERT INTO notes(id, vault_id, path, title, modified_unix, content_hash, human_relevance_score) VALUES({}, {}, {}, {}, {}, {}, {:.3});",
                sql_quote(&note.id),
                sql_quote(&scan.vault_id),
                sql_quote(&note.path),
                sql_quote(&note.title),
                note.modified_unix,
                sql_quote(&note.content_hash),
                note.human_relevance_score
            )
            .map_err(|err| format!("write note row: {err}"))?;
            for (key, value) in &note.frontmatter {
                writeln!(
                    stdin,
                    "INSERT INTO frontmatter(note_id, key, value) VALUES({}, {}, {});",
                    sql_quote(&note.id),
                    sql_quote(key),
                    sql_quote(value)
                )
                .map_err(|err| format!("write frontmatter row: {err}"))?;
            }
            for tag in &note.tags {
                writeln!(
                    stdin,
                    "INSERT OR IGNORE INTO tags(note_id, tag) VALUES({}, {});",
                    sql_quote(&note.id),
                    sql_quote(tag)
                )
                .map_err(|err| format!("write tag row: {err}"))?;
            }
            for link in &note.links {
                writeln!(
                    stdin,
                    "INSERT INTO links(source_note_id, target, raw) VALUES({}, {}, {});",
                    sql_quote(&note.id),
                    sql_quote(&link.target),
                    sql_quote(&link.raw)
                )
                .map_err(|err| format!("write link row: {err}"))?;
            }
            for section in &note.sections {
                writeln!(
                    stdin,
                    "INSERT INTO sections(id, note_id, heading_path, level, text, content_hash, human_relevance_score) VALUES({}, {}, {}, {}, {}, {}, {:.3});",
                    sql_quote(&section.id),
                    sql_quote(&note.id),
                    sql_quote(&section.heading_path),
                    section.level,
                    sql_quote(&section.text),
                    sql_quote(&section.content_hash),
                    section.human_relevance_score
                )
                .map_err(|err| format!("write section row: {err}"))?;
                writeln!(
                    stdin,
                    "INSERT INTO sections_fts(id, note_id, path, heading_path, text) VALUES({}, {}, {}, {}, {});",
                    sql_quote(&section.id),
                    sql_quote(&note.id),
                    sql_quote(&note.path),
                    sql_quote(&section.heading_path),
                    sql_quote(&section.text)
                )
                .map_err(|err| format!("write fts row: {err}"))?;
                writeln!(
                    stdin,
                    "INSERT INTO provenance(chunk_id, note_path, heading_path, content_hash, modified_unix, human_relevance_score) VALUES({}, {}, {}, {}, {}, {:.3});",
                    sql_quote(&section.id),
                    sql_quote(&note.path),
                    sql_quote(&section.heading_path),
                    sql_quote(&section.content_hash),
                    note.modified_unix,
                    section.human_relevance_score
                )
                .map_err(|err| format!("write provenance row: {err}"))?;
            }
        }
        writeln!(
            stdin,
            "INSERT INTO index_runs(id, vault_id, started_at_unix, notes_indexed) VALUES({}, {}, strftime('%s','now'), {});",
            sql_quote(&stable_id("run", &format!("{}:{}", scan.vault_id, scan.notes.len()))),
            sql_quote(&scan.vault_id),
            scan.notes.len()
        )
        .map_err(|err| format!("write index run row: {err}"))?;
        stdin
            .write_all(b"COMMIT;\n")
            .map_err(|err| format!("write sqlite commit: {err}"))?;
    }
    let output = child
        .wait_with_output()
        .map_err(|err| format!("wait sqlite3: {err}"))?;
    if !output.status.success() {
        return Err(format!(
            "sqlite3 failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

pub fn sql_literal(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| match ch {
            '\0' | '\u{001a}' => ' ',
            _ => ch,
        })
        .collect::<String>();
    format!("'{}'", sanitized.replace('\'', "''"))
}

fn sql_quote(value: &str) -> String {
    sql_literal(value)
}

pub fn turso_pipeline_url(database_url: &str) -> Result<String, String> {
    let trimmed = database_url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err("TURSO_DATABASE_URL cannot be empty".to_string());
    }
    let base = if let Some(rest) = trimmed.strip_prefix("libsql://") {
        format!("https://{rest}")
    } else if trimmed.starts_with("https://") || trimmed.starts_with("http://") {
        trimmed.to_string()
    } else {
        return Err(
            "TURSO_DATABASE_URL must start with libsql://, https://, or http://".to_string(),
        );
    };
    if base.ends_with("/v2/pipeline") {
        Ok(base)
    } else {
        Ok(format!("{base}/v2/pipeline"))
    }
}

pub fn turso_pipeline_request_json(statements: &[String]) -> String {
    let requests = statements
        .iter()
        .map(|sql| {
            format!(
                "{{\"type\":\"execute\",\"stmt\":{{\"sql\":{}}}}}",
                json_string(sql)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("{{\"requests\":[{requests}]}}")
}

pub const DUCKDB_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS vaults (
  id TEXT PRIMARY KEY,
  root_path TEXT NOT NULL,
  indexed_at_unix BIGINT NOT NULL
);
CREATE TABLE IF NOT EXISTS notes (
  id TEXT PRIMARY KEY,
  vault_id TEXT NOT NULL,
  path TEXT NOT NULL,
  title TEXT NOT NULL,
  modified_unix BIGINT NOT NULL,
  content_hash TEXT NOT NULL,
  human_relevance_score DOUBLE NOT NULL
);
CREATE TABLE IF NOT EXISTS sections (
  id TEXT PRIMARY KEY,
  note_id TEXT NOT NULL,
  heading_path TEXT NOT NULL,
  level INTEGER NOT NULL,
  text TEXT NOT NULL,
  content_hash TEXT NOT NULL,
  human_relevance_score DOUBLE NOT NULL
);
CREATE TABLE IF NOT EXISTS frontmatter (note_id TEXT NOT NULL, key TEXT NOT NULL, value TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS tags (note_id TEXT NOT NULL, tag TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS links (source_note_id TEXT NOT NULL, target TEXT NOT NULL, raw TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS provenance (
  chunk_id TEXT PRIMARY KEY,
  note_path TEXT NOT NULL,
  heading_path TEXT NOT NULL,
  content_hash TEXT NOT NULL,
  modified_unix BIGINT NOT NULL,
  human_relevance_score DOUBLE NOT NULL
);
CREATE TABLE IF NOT EXISTS embeddings (chunk_id TEXT PRIMARY KEY, model TEXT NOT NULL, dimensions INTEGER NOT NULL, vector_json TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS index_runs (id TEXT PRIMARY KEY, vault_id TEXT NOT NULL, started_at_unix BIGINT NOT NULL, notes_indexed BIGINT NOT NULL);
CREATE INDEX IF NOT EXISTS idx_notes_path ON notes(path);
CREATE INDEX IF NOT EXISTS idx_sections_note_id ON sections(note_id);
CREATE INDEX IF NOT EXISTS idx_sections_relevance ON sections(human_relevance_score);
"#;

pub fn duckdb_sync_statements(scan: &VaultScan, vault_root: &Path) -> Vec<String> {
    let mut statements = schema_statements_from(DUCKDB_SCHEMA);
    statements.push("BEGIN TRANSACTION".to_string());
    statements.push("DELETE FROM embeddings".to_string());
    statements.push("DELETE FROM provenance".to_string());
    statements.push("DELETE FROM links".to_string());
    statements.push("DELETE FROM tags".to_string());
    statements.push("DELETE FROM frontmatter".to_string());
    statements.push("DELETE FROM sections".to_string());
    statements.push("DELETE FROM notes".to_string());
    statements.push("DELETE FROM vaults".to_string());
    statements.push(format!(
        "INSERT INTO vaults(id, root_path, indexed_at_unix) VALUES({}, {}, epoch(CAST(now() AS TIMESTAMP)))",
        sql_quote(&scan.vault_id),
        sql_quote(&vault_root.to_string_lossy())
    ));
    append_row_statements(scan, &mut statements, false);
    statements.push(format!(
        "INSERT INTO index_runs(id, vault_id, started_at_unix, notes_indexed) VALUES({}, {}, epoch(CAST(now() AS TIMESTAMP)), {})",
        sql_quote(&stable_id("run", &format!("{}:{}", scan.vault_id, scan.notes.len()))),
        sql_quote(&scan.vault_id),
        scan.notes.len()
    ));
    statements.push("COMMIT".to_string());
    statements
}

pub fn turso_sync_statements(scan: &VaultScan, vault_root: &Path) -> Vec<String> {
    let mut statements = schema_statements();
    statements.push("BEGIN".to_string());
    statements.push("DELETE FROM sections_fts".to_string());
    statements.push("DELETE FROM vaults".to_string());
    statements.push(format!(
        "INSERT INTO vaults(id, root_path, indexed_at_unix) VALUES({}, {}, strftime('%s','now'))",
        sql_quote(&scan.vault_id),
        sql_quote(&vault_root.to_string_lossy())
    ));
    for note in &scan.notes {
        statements.push(format!(
            "INSERT INTO notes(id, vault_id, path, title, modified_unix, content_hash, human_relevance_score) VALUES({}, {}, {}, {}, {}, {}, {:.3})",
            sql_quote(&note.id),
            sql_quote(&scan.vault_id),
            sql_quote(&note.path),
            sql_quote(&note.title),
            note.modified_unix,
            sql_quote(&note.content_hash),
            note.human_relevance_score
        ));
        for (key, value) in &note.frontmatter {
            statements.push(format!(
                "INSERT INTO frontmatter(note_id, key, value) VALUES({}, {}, {})",
                sql_quote(&note.id),
                sql_quote(key),
                sql_quote(value)
            ));
        }
        for tag in &note.tags {
            statements.push(format!(
                "INSERT OR IGNORE INTO tags(note_id, tag) VALUES({}, {})",
                sql_quote(&note.id),
                sql_quote(tag)
            ));
        }
        for link in &note.links {
            statements.push(format!(
                "INSERT INTO links(source_note_id, target, raw) VALUES({}, {}, {})",
                sql_quote(&note.id),
                sql_quote(&link.target),
                sql_quote(&link.raw)
            ));
        }
        for section in &note.sections {
            statements.push(format!(
                "INSERT INTO sections(id, note_id, heading_path, level, text, content_hash, human_relevance_score) VALUES({}, {}, {}, {}, {}, {}, {:.3})",
                sql_quote(&section.id),
                sql_quote(&note.id),
                sql_quote(&section.heading_path),
                section.level,
                sql_quote(&section.text),
                sql_quote(&section.content_hash),
                section.human_relevance_score
            ));
            statements.push(format!(
                "INSERT INTO sections_fts(id, note_id, path, heading_path, text) VALUES({}, {}, {}, {}, {})",
                sql_quote(&section.id),
                sql_quote(&note.id),
                sql_quote(&note.path),
                sql_quote(&section.heading_path),
                sql_quote(&section.text)
            ));
            statements.push(format!(
                "INSERT INTO provenance(chunk_id, note_path, heading_path, content_hash, modified_unix, human_relevance_score) VALUES({}, {}, {}, {}, {}, {:.3})",
                sql_quote(&section.id),
                sql_quote(&note.path),
                sql_quote(&section.heading_path),
                sql_quote(&section.content_hash),
                note.modified_unix,
                section.human_relevance_score
            ));
        }
    }
    statements.push(format!(
        "INSERT INTO index_runs(id, vault_id, started_at_unix, notes_indexed) VALUES({}, {}, strftime('%s','now'), {})",
        sql_quote(&stable_id("run", &format!("{}:{}", scan.vault_id, scan.notes.len()))),
        sql_quote(&scan.vault_id),
        scan.notes.len()
    ));
    statements.push("COMMIT".to_string());
    statements
}

fn append_row_statements(
    scan: &VaultScan,
    statements: &mut Vec<String>,
    include_sections_fts: bool,
) {
    for note in &scan.notes {
        statements.push(format!(
            "INSERT INTO notes(id, vault_id, path, title, modified_unix, content_hash, human_relevance_score) VALUES({}, {}, {}, {}, {}, {}, {:.3})",
            sql_quote(&note.id),
            sql_quote(&scan.vault_id),
            sql_quote(&note.path),
            sql_quote(&note.title),
            note.modified_unix,
            sql_quote(&note.content_hash),
            note.human_relevance_score
        ));
        for (key, value) in &note.frontmatter {
            statements.push(format!(
                "INSERT INTO frontmatter(note_id, key, value) VALUES({}, {}, {})",
                sql_quote(&note.id),
                sql_quote(key),
                sql_quote(value)
            ));
        }
        for tag in &note.tags {
            statements.push(format!(
                "INSERT INTO tags(note_id, tag) VALUES({}, {})",
                sql_quote(&note.id),
                sql_quote(tag)
            ));
        }
        for link in &note.links {
            statements.push(format!(
                "INSERT INTO links(source_note_id, target, raw) VALUES({}, {}, {})",
                sql_quote(&note.id),
                sql_quote(&link.target),
                sql_quote(&link.raw)
            ));
        }
        for section in &note.sections {
            statements.push(format!(
                "INSERT INTO sections(id, note_id, heading_path, level, text, content_hash, human_relevance_score) VALUES({}, {}, {}, {}, {}, {}, {:.3})",
                sql_quote(&section.id),
                sql_quote(&note.id),
                sql_quote(&section.heading_path),
                section.level,
                sql_quote(&section.text),
                sql_quote(&section.content_hash),
                section.human_relevance_score
            ));
            if include_sections_fts {
                statements.push(format!(
                    "INSERT INTO sections_fts(id, note_id, path, heading_path, text) VALUES({}, {}, {}, {}, {})",
                    sql_quote(&section.id),
                    sql_quote(&note.id),
                    sql_quote(&note.path),
                    sql_quote(&section.heading_path),
                    sql_quote(&section.text)
                ));
            }
            statements.push(format!(
                "INSERT INTO provenance(chunk_id, note_path, heading_path, content_hash, modified_unix, human_relevance_score) VALUES({}, {}, {}, {}, {}, {:.3})",
                sql_quote(&section.id),
                sql_quote(&note.path),
                sql_quote(&section.heading_path),
                sql_quote(&section.content_hash),
                note.modified_unix,
                section.human_relevance_score
            ));
        }
    }
}

fn schema_statements() -> Vec<String> {
    schema_statements_from(SQLITE_SCHEMA)
}

fn schema_statements_from(schema: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut current = String::new();
    for line in schema.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("--") {
            continue;
        }
        if !current.is_empty() {
            current.push('\n');
        }
        current.push_str(line);
        if trimmed.ends_with(';') {
            statements.push(current.trim().trim_end_matches(';').to_string());
            current.clear();
        }
    }
    if !current.trim().is_empty() {
        statements.push(current.trim().to_string());
    }
    statements
}

pub fn json_string(value: &str) -> String {
    let mut out = String::from("\"");
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}

/// Deterministic tiny embedding provider for tests and offline smoke runs.
///
/// This is not semantically useful. It proves the provider/storage/query boundary
/// without sending private vault text to an external API.
pub fn deterministic_embedding(text: &str, dimensions: usize) -> Vec<f32> {
    let mut vector = vec![0.0_f32; dimensions.max(1)];
    for (index, byte) in text.as_bytes().iter().enumerate() {
        let slot = index % vector.len();
        vector[slot] += f32::from(*byte) / 255.0;
    }
    normalize(&mut vector);
    vector
}

pub fn embedding_to_json(vector: &[f32]) -> String {
    let values = vector
        .iter()
        .map(|value| format!("{value:.6}"))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{values}]")
}

pub fn embedding_from_json(value: &str) -> Vec<f32> {
    value
        .trim_matches(|ch| ch == '[' || ch == ']')
        .split(',')
        .filter_map(|part| part.trim().parse::<f32>().ok())
        .collect()
}

pub fn cosine_similarity(left: &[f32], right: &[f32]) -> f32 {
    if left.is_empty() || right.is_empty() || left.len() != right.len() {
        return 0.0;
    }
    left.iter().zip(right.iter()).map(|(a, b)| a * b).sum()
}

pub fn retrieval_text_quality_score(text: &str) -> f32 {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return 0.0;
    }
    let word_count = trimmed
        .split_whitespace()
        .filter(|part| part.chars().any(|ch| ch.is_alphanumeric()))
        .count();
    let char_count = trimmed.chars().filter(|ch| !ch.is_whitespace()).count();
    let unique_words = {
        let mut words = trimmed
            .split_whitespace()
            .map(|part| {
                part.trim_matches(|ch: char| !ch.is_alphanumeric())
                    .to_lowercase()
            })
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>();
        words.sort();
        words.dedup();
        words.len()
    };
    let mut score: f32 = 1.0;
    let lower = trimmed.to_lowercase();
    let url_count = lower.matches("http://").count() + lower.matches("https://").count();
    let email_marker_count = lower.matches('@').count();
    if lower.contains("switch to excalidraw view") || lower.contains("excalidraw view") {
        score *= 0.10;
    }
    if lower.starts_with("**aan:**") || lower.starts_with("aan:") || lower.starts_with("from:") {
        score *= 0.35;
    }
    if url_count >= 1 {
        score *= 0.55;
    }
    if email_marker_count >= 3 {
        score *= 0.60;
    }
    if word_count < 3 || char_count < 20 {
        score *= 0.15;
    } else if word_count < 8 || char_count < 60 {
        score *= 0.45;
    }
    if unique_words <= 2 {
        score *= 0.35;
    }
    score.clamp(0.05, 1.0)
}

fn normalize(vector: &mut [f32]) {
    let magnitude = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
    if magnitude > 0.0 {
        for value in vector {
            *value /= magnitude;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;

    #[test]
    fn default_database_path_uses_external_state_dir() {
        let config = RuntimeConfig::new(
            "/tmp/example-vault",
            Some(PathBuf::from("/tmp/vault-layer-state")),
        )
        .expect("valid config");
        assert_eq!(
            config.database_path("demo"),
            PathBuf::from("/tmp/vault-layer-state/demo/vault-layer.db")
        );
    }

    #[test]
    fn rejects_state_dir_inside_vault() {
        let error = RuntimeConfig::new(
            "/tmp/example-vault",
            Some(PathBuf::from("/tmp/example-vault/.vault-layer")),
        )
        .expect_err("state inside vault must be rejected");
        assert!(error.contains("outside the vault"));
    }

    #[test]
    fn command_surface_mentions_first_mvp_commands() {
        for command in ["init", "index", "search", "context", "serve"] {
            assert!(COMMANDS.contains(&command));
        }
    }

    #[test]
    fn retrieval_text_quality_downranks_status_only_chunks() {
        let status_only = retrieval_text_quality_score("Bezorgd.");
        let informative = retrieval_text_quality_score(
            "VaultLayer indexes Markdown sections with provenance, context, and retrieval signals.",
        );
        assert!(status_only < 0.2);
        assert!(informative > 0.9);
        assert!(informative > status_only * 5.0);
    }

    #[test]
    fn retrieval_text_quality_downranks_boilerplate_urls_and_email_headers() {
        let excalidraw = retrieval_text_quality_score(
            "==⚠ Switch to EXCALIDRAW VIEW in the MORE OPTIONS menu of this document. ⚠==",
        );
        let bookmark = retrieval_text_quality_score(
            "- [MB14 vs SARO | Grand Beatbox LOOPSTATION Battle 2017 | SEMI FINAL - YouTube](https://www.youtube.com/watch?v=demo)",
        );
        let email_header = retrieval_text_quality_score(
            "**Aan:** user@example.com, other@example.com, third@example.com\nSubject: hello",
        );
        assert!(excalidraw < 0.2);
        assert!(bookmark < 0.7);
        assert!(email_header < 0.4);
    }

    #[test]
    fn parses_frontmatter_headings_wikilinks_and_tags() {
        let dir = env::temp_dir().join(format!("vault-layer-test-{}", stable_hash("parse")));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("Projects")).expect("create fixture dir");
        let note_path = dir.join("Projects/Test Note.md");
        let mut file = File::create(note_path).expect("create note");
        writeln!(
            file,
            "---
title: Agent Vault
type: project
human_relevance_score: 0.8
---
# Intro
Hello [[Other Note|alias]] #project/agent
## Next
More text"
        )
        .expect("write note");
        drop(file);

        let scan = scan_vault(&dir).expect("scan vault");
        assert_eq!(scan.notes.len(), 1);
        let note = &scan.notes[0];
        assert_eq!(note.path, "Projects/Test Note.md");
        assert_eq!(note.title, "Agent Vault");
        assert!(note
            .frontmatter
            .contains(&("type".to_string(), "project".to_string())));
        assert_eq!(note.human_relevance_score, 0.8);
        assert_eq!(note.sections[0].human_relevance_score, 0.8);
        assert_eq!(note.links[0].target, "Other Note");
        assert!(note.tags.contains(&"project/agent".to_string()));
        assert_eq!(note.sections.len(), 2);
        assert_eq!(note.sections[0].heading_path, "Intro");
        assert_eq!(note.sections[1].heading_path, "Next");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn scanner_ignores_hidden_runtime_directories() {
        let dir = env::temp_dir().join(format!("vault-layer-hidden-{}", stable_hash("hidden")));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join(".stversions")).expect("create hidden dir");
        fs::create_dir_all(dir.join("Notes")).expect("create notes dir");
        fs::write(
            dir.join(".stversions/private.md"),
            "# Hidden\nshould not index",
        )
        .expect("write hidden");
        fs::write(dir.join("Notes/public.md"), "# Public\nshould index").expect("write visible");

        let scan = scan_vault(&dir).expect("scan vault");
        assert_eq!(scan.notes.len(), 1);
        assert_eq!(scan.notes[0].path, "Notes/public.md");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn writes_control_char_notes_without_sql_parse_errors() {
        let dir = env::temp_dir().join(format!("vault-layer-control-{}", stable_hash("control")));
        let state = env::temp_dir().join(format!(
            "vault-layer-control-state-{}",
            stable_hash("control-state")
        ));
        let _ = fs::remove_dir_all(&dir);
        let _ = fs::remove_dir_all(&state);
        fs::create_dir_all(&dir).expect("create vault dir");
        fs::write(
            dir.join("control.md"),
            b"# Control\ncontains nul \0 and sub \x1a plus quote ' safely",
        )
        .expect("write note");

        let scan = scan_vault(&dir).expect("scan vault");
        let db_path = state.join("demo/vault-layer.db");
        write_scan_sqlite(&scan, &dir, &db_path).expect("write sqlite with control chars");

        let _ = fs::remove_dir_all(&dir);
        let _ = fs::remove_dir_all(&state);
    }

    #[test]
    fn duplicate_headings_with_same_text_get_unique_section_ids() {
        let dir = env::temp_dir().join(format!("vault-layer-dupes-{}", stable_hash("dupes")));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create vault dir");
        fs::write(
            dir.join("dupes.md"),
            "# Repeat\nsame text\n# Repeat\nsame text",
        )
        .expect("write note");

        let scan = scan_vault(&dir).expect("scan vault");
        let sections = &scan.notes[0].sections;
        assert_eq!(sections.len(), 2);
        assert_ne!(sections[0].id, sections[1].id);

        let state = env::temp_dir().join(format!(
            "vault-layer-dupes-state-{}",
            stable_hash("dupes-state")
        ));
        let db_path = state.join("demo/vault-layer.db");
        write_scan_sqlite(&scan, &dir, &db_path).expect("write sqlite without duplicate ids");

        let _ = fs::remove_dir_all(&dir);
        let _ = fs::remove_dir_all(&state);
    }

    #[test]
    fn human_relevance_defaults_system_paths_lower() {
        let dir = env::temp_dir().join(format!("vault-layer-human-{}", stable_hash("human")));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("system")).expect("create system dir");
        fs::write(dir.join("system/agent.md"), "# Agent\nsystem context").expect("write note");

        let scan = scan_vault(&dir).expect("scan vault");
        assert_eq!(scan.notes[0].human_relevance_score, 0.25);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn writes_scan_to_sqlite_outside_repo_style_path() {
        let dir = env::temp_dir().join(format!("vault-layer-db-vault-{}", stable_hash("db-vault")));
        let state =
            env::temp_dir().join(format!("vault-layer-db-state-{}", stable_hash("db-state")));
        let _ = fs::remove_dir_all(&dir);
        let _ = fs::remove_dir_all(&state);
        fs::create_dir_all(&dir).expect("create vault dir");
        fs::write(
            dir.join("note.md"),
            "# Hello
SQLite shadow DB [[Target]] #db",
        )
        .expect("write note");
        let scan = scan_vault(&dir).expect("scan");
        let db_path = state.join("demo/vault-layer.db");
        write_scan_sqlite(&scan, &dir, &db_path).expect("write sqlite");
        assert!(db_path.exists());
        assert!(!is_inside(&db_path, &dir));
        let _ = fs::remove_dir_all(&dir);
        let _ = fs::remove_dir_all(&state);
    }

    #[test]
    fn local_duckdb_is_analytics_sidecar_backend() {
        let config = StorageBackendConfig::local_duckdb();
        assert_eq!(config.kind, StorageBackendKind::LocalDuckdb);
        assert_eq!(config.backend_name(), "duckdb");
        assert_eq!(config.index_write_mode(), "implemented-analytics-sidecar");
        assert_eq!(
            config.vector_mode(),
            "duckdb-analytics-portable-json-cosine"
        );
        assert!(config.database_url.is_none());
        assert!(!config.auth_token_present);
    }

    #[test]
    fn local_libsql_needs_no_url_or_token() {
        let config = StorageBackendConfig::local_libsql();
        assert_eq!(config.kind, StorageBackendKind::LocalLibsql);
        assert_eq!(config.backend_name(), "libsql-local");
        assert_eq!(
            config.index_write_mode(),
            "implemented-local-open-source-libsql"
        );
        assert_eq!(config.vector_mode(), "portable-json-cosine-on-libsql");
        assert!(config.database_url.is_none());
        assert!(!config.auth_token_present);
    }

    #[test]
    fn turso_pipeline_url_maps_libsql_to_http_api() {
        assert_eq!(
            turso_pipeline_url("libsql://demo-viggo.turso.io").expect("url"),
            "https://demo-viggo.turso.io/v2/pipeline"
        );
    }

    #[test]
    fn turso_pipeline_request_contains_executable_sql() {
        let body = turso_pipeline_request_json(&[
            "CREATE TABLE demo(id TEXT)".to_string(),
            "INSERT INTO demo(id) VALUES('a')".to_string(),
        ]);
        assert!(body.contains("\"type\":\"execute\""));
        assert!(body.contains("CREATE TABLE demo"));
        assert!(body.contains("INSERT INTO demo"));
    }

    #[test]
    fn local_sqlite_is_default_backend() {
        let config = StorageBackendConfig::local_sqlite();
        assert_eq!(config.backend_name(), "sqlite");
        assert_eq!(
            config.index_write_mode(),
            "implemented-primary-local-retrieval"
        );
        assert_eq!(
            config.vector_mode(),
            "sqlite-vec-target-json-cosine-fallback"
        );
    }

    #[test]
    fn turso_backend_is_explicit_and_vector_ready_target() {
        let config = StorageBackendConfig {
            kind: StorageBackendKind::TursoRemote,
            database_url: Some("libsql://example.turso.io".to_string()),
            auth_token_present: true,
        };
        assert_eq!(config.backend_name(), "turso-libsql");
        assert_eq!(
            config.index_write_mode(),
            "implemented-explicit-remote-sync"
        );
        assert_eq!(config.vector_mode(), "native-libsql-vector-target");
        assert!(LIBSQL_VECTOR_TARGET_SQL.contains("F32_BLOB"));
        assert!(LIBSQL_VECTOR_TARGET_SQL.contains("libsql_vector_idx"));
    }

    #[test]
    fn deterministic_embeddings_are_stable() {
        let first = deterministic_embedding("hello vault", 8);
        let second = deterministic_embedding("hello vault", 8);
        assert_eq!(first, second);
        assert_eq!(first.len(), 8);
        assert!(cosine_similarity(&first, &second) > 0.99);
        assert_eq!(embedding_from_json(&embedding_to_json(&first)).len(), 8);
    }
}
