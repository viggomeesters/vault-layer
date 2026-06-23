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
}
