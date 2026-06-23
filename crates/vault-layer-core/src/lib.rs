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
pub const COMMANDS: &[&str] = &["init", "index", "search", "context", "serve"];

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
        Ok(Self { vault_path, state_dir })
    }

    pub fn database_path(&self, vault_id: &str) -> PathBuf {
        self.state_dir.join(vault_id).join("vault-layer.db")
    }
}

pub fn default_state_dir() -> Result<PathBuf, String> {
    match env::var_os("VAULT_LAYER_STATE_DIR") {
        Some(value) if !value.is_empty() => Ok(PathBuf::from(value)),
        _ => {
            let home = env::var_os("HOME").ok_or_else(|| "HOME is not set; pass --state-dir".to_string())?;
            Ok(PathBuf::from(home).join(DEFAULT_STATE_SUBDIR))
        }
    }
}

pub fn is_inside(child: &Path, parent: &Path) -> bool {
    let child_components: Vec<_> = child.components().collect();
    let parent_components: Vec<_> = parent.components().collect();
    child_components.starts_with(&parent_components)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteRecord {
    pub id: String,
    pub path: String,
    pub title: String,
    pub modified_unix: u64,
    pub content_hash: String,
    pub frontmatter: Vec<(String, String)>,
    pub sections: Vec<SectionRecord>,
    pub links: Vec<LinkRecord>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionRecord {
    pub id: String,
    pub note_id: String,
    pub heading_path: String,
    pub level: u8,
    pub text: String,
    pub content_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkRecord {
    pub source_note_id: String,
    pub target: String,
    pub raw: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultScan {
    pub vault_id: String,
    pub notes: Vec<NoteRecord>,
}

pub fn scan_vault(vault_path: &Path) -> Result<VaultScan, String> {
    let mut files = Vec::new();
    collect_markdown_files(vault_path, vault_path, &mut files)?;
    files.sort();
    let vault_id = stable_id("vault", &vault_path.to_string_lossy());
    let mut notes = Vec::new();
    for relative in files {
        let absolute = vault_path.join(&relative);
        let content = fs::read_to_string(&absolute).map_err(|err| format!("read {}: {err}", absolute.display()))?;
        let metadata = fs::metadata(&absolute).map_err(|err| format!("metadata {}: {err}", absolute.display()))?;
        notes.push(parse_note(&vault_id, &relative, &content, &metadata)?);
    }
    Ok(VaultScan { vault_id, notes })
}

fn collect_markdown_files(root: &Path, current: &Path, out: &mut Vec<String>) -> Result<(), String> {
    for entry in fs::read_dir(current).map_err(|err| format!("read_dir {}: {err}", current.display()))? {
        let entry = entry.map_err(|err| format!("dir entry: {err}"))?;
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name == ".git" || name == ".obsidian" || name == ".trash" || name == "node_modules" {
            continue;
        }
        if path.is_dir() {
            collect_markdown_files(root, &path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "md") {
            let relative = path.strip_prefix(root).map_err(|err| err.to_string())?;
            out.push(relative.to_string_lossy().replace('\\', "/"));
        }
    }
    Ok(())
}

pub fn parse_note(vault_id: &str, relative_path: &str, content: &str, metadata: &fs::Metadata) -> Result<NoteRecord, String> {
    let content_hash = stable_hash(content);
    let note_id = stable_id("note", &format!("{vault_id}:{relative_path}:{content_hash}"));
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
    let sections = parse_sections(&note_id, relative_path, body);
    let links = extract_wikilinks(&note_id, content);
    let tags = extract_tags(content);
    Ok(NoteRecord { id: note_id, path: relative_path.to_string(), title, modified_unix, content_hash, frontmatter, sections, links, tags })
}

fn parse_frontmatter(content: &str) -> (Vec<(String, String)>, &str) {
    if !content.starts_with("---
") {
        return (Vec::new(), content);
    }
    let rest = &content[4..];
    if let Some(end) = rest.find("
---
") {
        let raw = &rest[..end];
        let body = &rest[end + 5..];
        let pairs = raw
            .lines()
            .filter_map(|line| line.split_once(':'))
            .map(|(key, value)| (key.trim().to_string(), value.trim().trim_matches('"').to_string()))
            .collect();
        (pairs, body)
    } else {
        (Vec::new(), content)
    }
}

fn parse_sections(note_id: &str, relative_path: &str, body: &str) -> Vec<SectionRecord> {
    let mut sections = Vec::new();
    let mut current_heading = String::from("root");
    let mut current_level = 0_u8;
    let mut buffer = String::new();
    for line in body.lines() {
        if let Some((level, heading)) = parse_heading(line) {
            push_section(&mut sections, note_id, relative_path, &current_heading, current_level, &buffer);
            current_heading = heading;
            current_level = level;
            buffer.clear();
        } else {
            buffer.push_str(line);
            buffer.push('\n');
        }
    }
    push_section(&mut sections, note_id, relative_path, &current_heading, current_level, &buffer);
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

fn push_section(sections: &mut Vec<SectionRecord>, note_id: &str, relative_path: &str, heading: &str, level: u8, text: &str) {
    let trimmed = text.trim();
    if trimmed.is_empty() && heading == "root" {
        return;
    }
    let content_hash = stable_hash(trimmed);
    let id = stable_id("chunk", &format!("{note_id}:{relative_path}:{heading}:{content_hash}"));
    sections.push(SectionRecord { id, note_id: note_id.to_string(), heading_path: heading.to_string(), level, text: trimmed.to_string(), content_hash });
}

fn extract_wikilinks(note_id: &str, content: &str) -> Vec<LinkRecord> {
    let mut links = Vec::new();
    let mut rest = content;
    while let Some(start) = rest.find("[[") {
        rest = &rest[start + 2..];
        let Some(end) = rest.find("]]") else { break; };
        let raw_target = &rest[..end];
        let target = raw_target.split('|').next().unwrap_or(raw_target).trim().to_string();
        links.push(LinkRecord { source_note_id: note_id.to_string(), target, raw: format!("[[{raw_target}]]") });
        rest = &rest[end + 2..];
    }
    links
}

fn extract_tags(content: &str) -> Vec<String> {
    let mut tags = Vec::new();
    for token in content.split_whitespace() {
        let token = token.trim_matches(|ch: char| ch == ',' || ch == '.' || ch == ';' || ch == ')' || ch == '(');
        if let Some(tag) = token.strip_prefix('#') {
            if !tag.is_empty() && tag.chars().all(|ch| ch.is_alphanumeric() || ch == '-' || ch == '_' || ch == '/') {
                let tag = tag.to_string();
                if !tags.contains(&tag) {
                    tags.push(tag);
                }
            }
        }
    }
    tags
}

fn title_from_path(path: &str) -> String {
    Path::new(path).file_stem().and_then(|stem| stem.to_str()).unwrap_or(path).replace('-', " ")
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
pub fn write_scan_sqlite(scan: &VaultScan, vault_root: &Path, db_path: &Path) -> Result<(), String> {
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("create state dir {}: {err}", parent.display()))?;
    }
    let mut sql = String::new();
    sql.push_str(SQLITE_SCHEMA);
    sql.push_str("
BEGIN;
");
    sql.push_str("DELETE FROM sections_fts;
DELETE FROM vaults;
");
    sql.push_str(&format!(
        "INSERT INTO vaults(id, root_path, indexed_at_unix) VALUES({}, {}, strftime('%s','now'));
",
        sql_quote(&scan.vault_id),
        sql_quote(&vault_root.to_string_lossy())
    ));
    for note in &scan.notes {
        sql.push_str(&format!(
            "INSERT INTO notes(id, vault_id, path, title, modified_unix, content_hash) VALUES({}, {}, {}, {}, {}, {});
",
            sql_quote(&note.id), sql_quote(&scan.vault_id), sql_quote(&note.path), sql_quote(&note.title), note.modified_unix, sql_quote(&note.content_hash)
        ));
        for (key, value) in &note.frontmatter {
            sql.push_str(&format!("INSERT INTO frontmatter(note_id, key, value) VALUES({}, {}, {});
", sql_quote(&note.id), sql_quote(key), sql_quote(value)));
        }
        for tag in &note.tags {
            sql.push_str(&format!("INSERT OR IGNORE INTO tags(note_id, tag) VALUES({}, {});
", sql_quote(&note.id), sql_quote(tag)));
        }
        for link in &note.links {
            sql.push_str(&format!("INSERT INTO links(source_note_id, target, raw) VALUES({}, {}, {});
", sql_quote(&note.id), sql_quote(&link.target), sql_quote(&link.raw)));
        }
        for section in &note.sections {
            sql.push_str(&format!(
                "INSERT INTO sections(id, note_id, heading_path, level, text, content_hash) VALUES({}, {}, {}, {}, {}, {});
",
                sql_quote(&section.id), sql_quote(&note.id), sql_quote(&section.heading_path), section.level, sql_quote(&section.text), sql_quote(&section.content_hash)
            ));
            sql.push_str(&format!(
                "INSERT INTO sections_fts(id, note_id, path, heading_path, text) VALUES({}, {}, {}, {}, {});
",
                sql_quote(&section.id), sql_quote(&note.id), sql_quote(&note.path), sql_quote(&section.heading_path), sql_quote(&section.text)
            ));
            sql.push_str(&format!(
                "INSERT INTO provenance(chunk_id, note_path, heading_path, content_hash, modified_unix) VALUES({}, {}, {}, {}, {});
",
                sql_quote(&section.id), sql_quote(&note.path), sql_quote(&section.heading_path), sql_quote(&section.content_hash), note.modified_unix
            ));
        }
    }
    sql.push_str(&format!(
        "INSERT INTO index_runs(id, vault_id, started_at_unix, notes_indexed) VALUES({}, {}, strftime('%s','now'), {});
",
        sql_quote(&stable_id("run", &format!("{}:{}", scan.vault_id, scan.notes.len()))),
        sql_quote(&scan.vault_id),
        scan.notes.len()
    ));
    sql.push_str("COMMIT;
");

    let mut child = std::process::Command::new("sqlite3")
        .arg(db_path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|err| format!("spawn sqlite3: {err}"))?;
    {
        use std::io::Write;
        let stdin = child.stdin.as_mut().ok_or_else(|| "sqlite3 stdin unavailable".to_string())?;
        stdin.write_all(sql.as_bytes()).map_err(|err| format!("write sqlite3 script: {err}"))?;
    }
    let output = child.wait_with_output().map_err(|err| format!("wait sqlite3: {err}"))?;
    if !output.status.success() {
        return Err(format!("sqlite3 failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    Ok(())
}

pub fn sql_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''''"))
}

fn sql_quote(value: &str) -> String {
    sql_literal(value)
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
    let values = vector.iter().map(|value| format!("{value:.6}")).collect::<Vec<_>>().join(",");
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
        let config = RuntimeConfig::new("/tmp/example-vault", Some(PathBuf::from("/tmp/vault-layer-state"))).expect("valid config");
        assert_eq!(config.database_path("demo"), PathBuf::from("/tmp/vault-layer-state/demo/vault-layer.db"));
    }

    #[test]
    fn rejects_state_dir_inside_vault() {
        let error = RuntimeConfig::new("/tmp/example-vault", Some(PathBuf::from("/tmp/example-vault/.vault-layer"))).expect_err("state inside vault must be rejected");
        assert!(error.contains("outside the vault"));
    }

    #[test]
    fn command_surface_mentions_first_mvp_commands() {
        for command in ["init", "index", "search", "context", "serve"] {
            assert!(COMMANDS.contains(&command));
        }
    }

    #[test]
    fn parses_frontmatter_headings_wikilinks_and_tags() {
        let dir = env::temp_dir().join(format!("vault-layer-test-{}", stable_hash("parse")));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("Projects")).expect("create fixture dir");
        let note_path = dir.join("Projects/Test Note.md");
        let mut file = File::create(note_path).expect("create note");
        writeln!(file, "---
title: Agent Vault
type: project
---
# Intro
Hello [[Other Note|alias]] #project/agent
## Next
More text").expect("write note");
        drop(file);

        let scan = scan_vault(&dir).expect("scan vault");
        assert_eq!(scan.notes.len(), 1);
        let note = &scan.notes[0];
        assert_eq!(note.path, "Projects/Test Note.md");
        assert_eq!(note.title, "Agent Vault");
        assert!(note.frontmatter.contains(&("type".to_string(), "project".to_string())));
        assert_eq!(note.links[0].target, "Other Note");
        assert!(note.tags.contains(&"project/agent".to_string()));
        assert_eq!(note.sections.len(), 2);
        assert_eq!(note.sections[0].heading_path, "Intro");
        assert_eq!(note.sections[1].heading_path, "Next");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn writes_scan_to_sqlite_outside_repo_style_path() {
        let dir = env::temp_dir().join(format!("vault-layer-db-vault-{}", stable_hash("db-vault")));
        let state = env::temp_dir().join(format!("vault-layer-db-state-{}", stable_hash("db-state")));
        let _ = fs::remove_dir_all(&dir);
        let _ = fs::remove_dir_all(&state);
        fs::create_dir_all(&dir).expect("create vault dir");
        fs::write(dir.join("note.md"), "# Hello
SQLite shadow DB [[Target]] #db").expect("write note");
        let scan = scan_vault(&dir).expect("scan");
        let db_path = state.join("demo/vault-layer.db");
        write_scan_sqlite(&scan, &dir, &db_path).expect("write sqlite");
        assert!(db_path.exists());
        assert!(!is_inside(&db_path, &dir));
        let _ = fs::remove_dir_all(&dir);
        let _ = fs::remove_dir_all(&state);
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
