use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use vault_layer_core::{
    cosine_similarity, default_state_dir, deterministic_embedding, duckdb_sync_statements,
    embedding_from_json, embedding_to_json, retrieval_text_quality_score, scan_vault_limited,
    scan_vault_limited_with_progress, sql_literal, turso_pipeline_request_json, turso_pipeline_url,
    turso_sync_statements, write_scan_sqlite_with_progress, RuntimeConfig, StorageBackendConfig,
    COMMANDS, DEFAULT_STATE_SUBDIR,
};
use vault_layer_sqlite_vec::{refresh_vec_embeddings, search_vec_embeddings};

fn main() {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        None | Some("-h") | Some("--help") => print_help(),
        Some("init") => init_command(args.collect()),
        Some("index") => index_command(args.collect()),
        Some("sync-turso") => sync_turso_command(args.collect()),
        Some("search") => search_command(args.collect()),
        Some("get-note") => get_note_command(args.collect()),
        Some("related") => related_command(args.collect()),
        Some("embed") => embed_command(args.collect()),
        Some("vector-search") => vector_search_command(args.collect()),
        Some("hybrid-search") => hybrid_search_command(args.collect()),
        Some("context") => context_command(args.collect()),
        Some("serve") => serve_command(args.collect()),
        Some("backend-info") => backend_info_command(),
        Some("doctor") => doctor_command(args.collect()),
        Some("sqlite-vec-info") => sqlite_vec_info_command(),
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
    let vault_path = args
        .first()
        .cloned()
        .unwrap_or_else(|| "<vault-path>".to_string());
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
    let backend = StorageBackendConfig::from_env();
    println!("backend={}", backend.backend_name());
    println!("index_write_mode={}", backend.index_write_mode());
    println!("vector_mode={}", backend.vector_mode());
    if backend.kind == vault_layer_core::StorageBackendKind::LocalSqlite {
        if let Ok(smoke) = vault_layer_sqlite_vec::sqlite_vec_smoke() {
            println!("sqlite_vec_available=true");
            println!("sqlite_vec_version={}", smoke.version);
        } else {
            println!("sqlite_vec_available=false");
        }
    }
}

fn sqlite_vec_info_command() {
    match vault_layer_sqlite_vec::sqlite_vec_smoke() {
        Ok(smoke) => {
            println!("sqlite_vec_available=true");
            println!("sqlite_vec_version={}", smoke.version);
            println!("sqlite_vec_dimensions={}", smoke.dimensions);
            println!("sqlite_vec_distance={}", smoke.distance);
            println!("vector_runtime=native-sqlite-vec-smoke");
        }
        Err(error) => {
            println!("sqlite_vec_available=false");
            println!("sqlite_vec_error={error}");
            println!("vector_runtime=json-cosine-fallback");
        }
    }
}

fn backend_info_command() {
    let backend = StorageBackendConfig::from_env();
    println!("backend={}", backend.backend_name());
    println!("database_url_configured={}", backend.database_url.is_some());
    println!("auth_token_configured={}", backend.auth_token_present);
    println!("index_write_mode={}", backend.index_write_mode());
    println!("vector_mode={}", backend.vector_mode());
    if backend.kind == vault_layer_core::StorageBackendKind::LocalSqlite {
        if let Ok(smoke) = vault_layer_sqlite_vec::sqlite_vec_smoke() {
            println!("sqlite_vec_available=true");
            println!("sqlite_vec_version={}", smoke.version);
        } else {
            println!("sqlite_vec_available=false");
        }
    }
    println!("local_indexing=true");
    println!(
        "remote_sync={}",
        if backend.kind == vault_layer_core::StorageBackendKind::TursoRemote {
            "implemented-explicit"
        } else {
            "not-configured"
        }
    );
}

fn doctor_command(args: Vec<String>) {
    let vault_arg = args
        .iter()
        .find(|arg| !arg.starts_with("--"))
        .map(PathBuf::from);
    let state_dir = state_dir_from_args(args.clone())
        .or_else(|| default_state_dir().ok())
        .unwrap_or_else(|| PathBuf::from(".vault-layer-state"));
    let cache_dir = fastembed_cache_dir();
    let backend = StorageBackendConfig::from_env();
    let mut ok = true;

    println!("vault_layer_doctor=1");
    println!("read_only_default=true");
    println!("state_dir={}", state_dir.display());
    println!("fastembed_cache_dir={}", cache_dir.display());
    println!("backend={}", backend.backend_name());
    println!(
        "remote_sync_disabled_by_default={}",
        backend.kind != vault_layer_core::StorageBackendKind::TursoRemote
    );
    if backend.kind == vault_layer_core::StorageBackendKind::TursoRemote {
        println!("ERROR remote backend configured; unset TURSO_DATABASE_URL/TURSO_AUTH_TOKEN or set VAULT_LAYER_BACKEND=sqlite for a local pilot");
        ok = false;
    }

    match vault_arg.as_ref() {
        Some(vault_path) => {
            println!("vault_path={}", vault_path.display());
            match fs::metadata(vault_path) {
                Ok(metadata) if metadata.is_dir() => println!("vault_readable=true"),
                Ok(_) => {
                    println!("ERROR vault_path_is_not_directory=true");
                    ok = false;
                }
                Err(error) => {
                    println!("ERROR vault_readable=false error={error}");
                    ok = false;
                }
            }
            if path_is_within(&state_dir, vault_path) || path_is_within(&cache_dir, vault_path) {
                println!("ERROR runtime_state_or_cache_inside_vault=true");
                ok = false;
            } else {
                println!("runtime_state_outside_vault=true");
            }
        }
        None => println!("vault_readable=skipped_no_vault_path"),
    }

    if path_is_within(&state_dir, Path::new(".")) || path_is_within(&cache_dir, Path::new(".")) {
        println!("ERROR runtime_state_or_cache_inside_repo=true");
        ok = false;
    } else {
        println!("runtime_state_outside_repo=true");
    }

    match fs::create_dir_all(&state_dir) {
        Ok(()) => println!("state_dir_writable=true"),
        Err(error) => {
            println!("ERROR state_dir_writable=false error={error}");
            ok = false;
        }
    }
    match fs::create_dir_all(&cache_dir) {
        Ok(()) => println!("fastembed_cache_writable=true"),
        Err(error) => {
            println!("ERROR fastembed_cache_writable=false error={error}");
            ok = false;
        }
    }

    match vault_layer_sqlite_vec::sqlite_vec_smoke() {
        Ok(smoke) => {
            println!("sqlite_vec_available=true");
            println!("sqlite_vec_version={}", smoke.version);
        }
        Err(error) => {
            println!("ERROR sqlite_vec_available=false error={error}");
            ok = false;
        }
    }

    match run_fastembed_helper(&["doctor smoke".to_string()], EmbeddingKind::Query) {
        Ok(batch) => {
            println!("fastembed_available=true");
            println!("fastembed_model={}", batch.model);
            println!("fastembed_dimensions={}", batch.dimensions);
        }
        Err(error) => {
            println!("ERROR fastembed_available=false error={error}");
            ok = false;
        }
    }

    println!("doctor_status={}", if ok { "ok" } else { "failed" });
    if !ok {
        std::process::exit(1);
    }
}

fn path_is_within(path: &Path, root: &Path) -> bool {
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    path.starts_with(root)
}

fn index_command(args: Vec<String>) {
    let backend = StorageBackendConfig::from_env();
    let options = CliOptions::parse(args.clone());
    if backend.kind == vault_layer_core::StorageBackendKind::TursoRemote {
        if options.remote_sync {
            sync_turso_command(args);
            return;
        }
        fail("Remote Turso/libSQL is configured. Pass --remote-sync to upload, or use VAULT_LAYER_BACKEND=libsql-local for a local open-source Turso DB.");
    }
    let vault_path = args
        .first()
        .cloned()
        .unwrap_or_else(|| "<vault-path>".to_string());
    let state_dir = state_dir_from_args(args);
    match RuntimeConfig::new(&vault_path, state_dir) {
        Ok(config) => {
            let scan = scan_vault_limited_with_progress(
                &config.vault_path,
                options.limit.map(|v| v as usize),
                print_progress,
            );
            match scan {
                Ok(scan) => {
                    let db_path = match backend.kind {
                        vault_layer_core::StorageBackendKind::LocalDuckdb => {
                            let db_path = config.duckdb_database_path(&scan.vault_id);
                            if let Err(error) =
                                write_scan_duckdb(&scan, &config.vault_path, &db_path)
                            {
                                fail(&format!("index failed: {error}"));
                            }
                            db_path
                        }
                        vault_layer_core::StorageBackendKind::LocalLibsql => {
                            let db_path = config.libsql_database_path(&scan.vault_id);
                            if let Err(error) =
                                write_scan_libsql_local(&scan, &config.vault_path, &db_path)
                            {
                                fail(&format!("index failed: {error}"));
                            }
                            db_path
                        }
                        _ => {
                            let db_path = config.database_path(&scan.vault_id);
                            if !options.force
                                && db_path.exists()
                                && existing_note_count(&db_path) == Some(scan.notes.len())
                            {
                                eprintln!(
                                    "vault-layer progress phase=write_skipped_existing count={}",
                                    scan.notes.len()
                                );
                            } else if let Err(error) = write_scan_sqlite_with_progress(
                                &scan,
                                &config.vault_path,
                                &db_path,
                                print_progress,
                            ) {
                                fail(&format!("index failed: {error}"));
                            }
                            db_path
                        }
                    };
                    println!("vault-layer index complete");
                    println!("vault_path={vault_path}");
                    println!("read_only=true");
                    println!("backend={}", backend.backend_name());
                    println!("notes_indexed={}", scan.notes.len());
                    println!("db_path={}", db_path.display());
                }
                Err(error) => fail(&format!("scan failed: {error}")),
            }
        }
        Err(error) => fail(&format!("config failed: {error}")),
    }
}

fn print_progress(phase: &str, count: usize) {
    if count <= 5 || count % 500 == 0 {
        eprintln!("vault-layer progress phase={phase} count={count}");
    }
}

fn existing_note_count(db_path: &Path) -> Option<usize> {
    let rows = sqlite_table(&db_path.to_path_buf(), "SELECT count(*) FROM notes");
    rows.first()?.first()?.parse().ok()
}

fn write_scan_duckdb(
    scan: &vault_layer_core::VaultScan,
    vault_root: &std::path::Path,
    db_path: &std::path::Path,
) -> Result<(), String> {
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("create duckdb dir: {err}"))?;
    }
    if db_path.exists() {
        fs::remove_file(db_path).map_err(|err| format!("replace duckdb db: {err}"))?;
    }
    let conn = duckdb::Connection::open(db_path).map_err(|err| format!("open duckdb: {err}"))?;
    for statement in duckdb_sync_statements(scan, vault_root) {
        conn.execute_batch(&statement)
            .map_err(|err| format!("execute duckdb: {err}; sql={statement}"))?;
    }
    Ok(())
}

fn write_scan_libsql_local(
    scan: &vault_layer_core::VaultScan,
    vault_root: &std::path::Path,
    db_path: &std::path::Path,
) -> Result<(), String> {
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("create libsql dir: {err}"))?;
    }
    if db_path.exists() {
        fs::remove_file(db_path).map_err(|err| format!("replace libsql db: {err}"))?;
    }
    let statements = turso_sync_statements(scan, vault_root);
    let runtime = tokio::runtime::Runtime::new().map_err(|err| format!("start tokio: {err}"))?;
    runtime.block_on(async {
        let db = libsql::Builder::new_local(db_path)
            .build()
            .await
            .map_err(|err| format!("open local libsql: {err}"))?;
        let conn = db
            .connect()
            .map_err(|err| format!("connect local libsql: {err}"))?;
        for statement in statements {
            conn.execute_batch(&statement)
                .await
                .map_err(|err| format!("execute local libsql: {err}; sql={statement}"))?;
        }
        Ok::<(), String>(())
    })
}

fn sync_turso_command(args: Vec<String>) {
    let backend = StorageBackendConfig::from_env();
    if backend.kind != vault_layer_core::StorageBackendKind::TursoRemote {
        fail("sync-turso requires TURSO_DATABASE_URL");
    }
    if !backend.auth_token_present {
        fail("sync-turso requires TURSO_AUTH_TOKEN");
    }
    let vault_path = args
        .first()
        .cloned()
        .unwrap_or_else(|| "<vault-path>".to_string());
    let options = CliOptions::parse(args.clone());
    let state_dir = state_dir_from_args(args);
    let config = RuntimeConfig::new(&vault_path, state_dir)
        .unwrap_or_else(|error| fail(&format!("config failed: {error}")));
    let scan = scan_vault_limited(&config.vault_path, options.limit.map(|v| v as usize))
        .unwrap_or_else(|error| fail(&format!("scan failed: {error}")));
    let url = turso_pipeline_url(backend.database_url.as_deref().unwrap_or_default())
        .unwrap_or_else(|error| fail(&format!("turso url failed: {error}")));
    let token =
        env::var("TURSO_AUTH_TOKEN").unwrap_or_else(|_| fail("TURSO_AUTH_TOKEN is not set"));
    let statements = turso_sync_statements(&scan, &config.vault_path);
    let batches = sync_turso_batches(&url, &token, &statements, 200)
        .unwrap_or_else(|error| fail(&format!("turso sync failed: {error}")));
    println!("vault-layer turso sync complete");
    println!("vault_path={vault_path}");
    println!("read_only=true");
    println!("notes_synced={}", scan.notes.len());
    println!("statements_sent={}", statements.len());
    println!("batches_sent={batches}");
    println!("backend=turso-libsql");
}

fn sync_turso_batches(
    url: &str,
    token: &str,
    statements: &[String],
    batch_size: usize,
) -> Result<usize, String> {
    if statements.is_empty() {
        return Ok(0);
    }
    let mut sent = 0usize;
    for (index, chunk) in statements.chunks(batch_size.max(1)).enumerate() {
        let body = turso_pipeline_request_json(chunk);
        let body_path = env::temp_dir().join(format!(
            "vault-layer-turso-{}-{index}.json",
            std::process::id()
        ));
        fs::write(&body_path, body).map_err(|err| format!("write request body: {err}"))?;
        let auth_header = ["Authori", "zation: ", "Bearer", " ", token].concat();
        let output = Command::new("curl")
            .arg("--fail-with-body")
            .arg("--silent")
            .arg("--show-error")
            .arg("--request")
            .arg("POST")
            .arg("--header")
            .arg("Content-Type: application/json")
            .arg("--header")
            .arg(auth_header)
            .arg("--data-binary")
            .arg(format!("@{}", body_path.display()))
            .arg(url)
            .output()
            .map_err(|err| format!("start curl: {err}"));
        let _ = fs::remove_file(&body_path);
        let output = output?;
        if !output.status.success() {
            return Err(format!(
                "curl batch {index} failed: {}{}",
                String::from_utf8_lossy(&output.stderr),
                String::from_utf8_lossy(&output.stdout)
            ));
        }
        sent += 1;
    }
    Ok(sent)
}

fn search_command(args: Vec<String>) {
    let (query, rest) = split_query(args);
    let options = CliOptions::parse(rest);
    let db = require_db(&options);
    let limit = options.limit.unwrap_or(10);
    let sql = if is_duckdb_path(&db) {
        let like_query = format!("%{}%", query);
        format!(
            "SELECT s.id AS chunk_id, n.path, s.heading_path, substr(s.text, 1, 240) AS excerpt, CAST(0.0 AS DOUBLE) AS score, s.content_hash, n.modified_unix, s.human_relevance_score \
             FROM sections s JOIN notes n ON n.id = s.note_id \
             WHERE s.text LIKE {} OR n.path LIKE {} OR s.heading_path LIKE {} \
             ORDER BY s.human_relevance_score DESC, n.modified_unix DESC LIMIT {};",
            sql_literal(&like_query), sql_literal(&like_query), sql_literal(&like_query), limit
        )
    } else {
        sqlite_fts_search_sql(&query, limit, 240)
    };
    print_query_json(&db, &sql);
}

fn get_note_command(args: Vec<String>) {
    let (needle, rest) = split_query(args);
    let options = CliOptions::parse(rest);
    let db = require_db(&options);
    let like = format!("%{}%", needle);
    let sql = format!(
        "SELECT n.id, n.path, n.title, n.modified_unix, n.content_hash, n.human_relevance_score, substr(group_concat(s.heading_path || ': ' || s.text, char(10)), 1, 4000) AS bounded_content \
         FROM notes n LEFT JOIN sections s ON s.note_id = n.id \
         WHERE n.id = {} OR n.path = {} OR n.path LIKE {} GROUP BY n.id ORDER BY n.path LIMIT 1;",
        sql_literal(&needle), sql_literal(&needle), sql_literal(&like)
    );
    print_query_json(&db, &sql);
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
    print_query_json(&db, &sql);
}

fn embed_command(args: Vec<String>) {
    let options = CliOptions::parse(args);
    let db = require_db(&options);
    let rows = sqlite_table(
        &db,
        "SELECT id, replace(replace(text, char(10), ' '), char(9), ' ') FROM sections ORDER BY id;",
    );
    let texts = rows
        .iter()
        .filter_map(|row| row.get(1).cloned())
        .collect::<Vec<_>>();
    let provider = EmbeddingProvider::from_model_option(options.embedding_model.as_deref())
        .unwrap_or_else(|error| fail(&error));
    let batch = provider
        .embed_batch(&texts, EmbeddingKind::Passage)
        .unwrap_or_else(|error| fail(&format!("embedding failed: {error}")));
    if batch.embeddings.len() != rows.len() {
        fail("embedding provider returned mismatched row count");
    }
    let mut sql = format!(
        "PRAGMA foreign_keys = ON;\nBEGIN; DELETE FROM embeddings WHERE model = {};\n",
        sql_literal(&batch.model)
    );
    for (row, embedding_json) in rows.iter().zip(batch.embeddings.iter()) {
        if row.len() < 2 {
            continue;
        }
        sql.push_str(&format!(
            "INSERT OR REPLACE INTO embeddings(chunk_id, model, dimensions, embedding_json, embedded_at_unix) VALUES({}, {}, {}, {}, strftime('%s','now'));\n",
            sql_literal(&row[0]),
            sql_literal(&batch.model),
            batch.dimensions,
            sql_literal(embedding_json)
        ));
    }
    sql.push_str("COMMIT;\n");
    run_sqlite(&db, &sql);
    let native_vec = refresh_vec_embeddings(&db, &batch.model, batch.dimensions).ok();
    let sqlite_vec_rows = native_vec.as_ref().map(|refresh| refresh.rows).unwrap_or(0);
    let vector_runtime = if native_vec.is_some() {
        "native-sqlite-vec+json-fallback"
    } else {
        "json-cosine-fallback"
    };
    println!(
        "{{\"model\":{},\"dimensions\":{},\"chunks_embedded\":{},\"sqlite_vec_rows\":{},\"vector_runtime\":\"{}\",\"cache_dir\":{}}}",
        json_string(&batch.model),
        batch.dimensions,
        rows.len(),
        sqlite_vec_rows,
        vector_runtime,
        batch.cache_dir
            .as_deref()
            .map(json_string)
            .unwrap_or_else(|| "null".to_string())
    );
}

fn vector_search_command(args: Vec<String>) {
    let (query, rest) = split_query(args);
    let options = CliOptions::parse(rest);
    let db = require_db(&options);
    let limit = options.limit.unwrap_or(10) as usize;
    let provider = EmbeddingProvider::from_model_option(options.embedding_model.as_deref())
        .unwrap_or_else(|error| fail(&error));
    let query_batch = provider
        .embed_batch(&[query.clone()], EmbeddingKind::Query)
        .unwrap_or_else(|error| fail(&format!("query embedding failed: {error}")));
    let query_embedding = query_batch
        .embeddings
        .first()
        .map(|embedding| embedding_from_json(embedding))
        .unwrap_or_default();

    if let Ok(hits) =
        search_vec_embeddings(&db, &query_embedding, limit.saturating_mul(8).max(limit))
    {
        if !hits.is_empty() {
            let mut scored = Vec::new();
            for hit in hits {
                let sql = format!(
                    "SELECT s.id, n.path, s.heading_path, substr(s.text, 1, 240), s.content_hash, n.modified_unix, s.human_relevance_score, s.text FROM sections s JOIN notes n ON n.id = s.note_id WHERE s.id = {} LIMIT 1;",
                    sql_literal(&hit.chunk_id)
                );
                for row in sqlite_table(&db, &sql) {
                    if row.len() < 8 {
                        continue;
                    }
                    let vector_score = 1.0_f32 / (1.0_f32 + hit.distance as f32);
                    let text_quality = retrieval_text_quality_score(&row[7]);
                    let score = vector_score * text_quality;
                    scored.push((score, vector_score, text_quality, hit.distance, row));
                }
            }
            scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
            print_vector_rows(scored.into_iter().take(limit), "native-sqlite-vec");
            return;
        }
    }

    let rows = sqlite_table(&db, &format!("SELECT e.chunk_id, n.path, s.heading_path, substr(s.text, 1, 240), s.content_hash, n.modified_unix, s.human_relevance_score, e.embedding_json, s.text FROM embeddings e JOIN sections s ON s.id = e.chunk_id JOIN notes n ON n.id = s.note_id WHERE e.model = {};", sql_literal(provider.model_name())));
    let mut scored = rows
        .into_iter()
        .filter_map(|row| {
            if row.len() < 9 {
                return None;
            }
            let embedding = embedding_from_json(&row[7]);
            let cosine = cosine_similarity(&query_embedding, &embedding);
            let text_quality = retrieval_text_quality_score(&row[8]);
            let score = cosine * text_quality;
            Some((score, cosine, text_quality, 0.0_f64, row))
        })
        .collect::<Vec<_>>();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    print_vector_rows(scored.into_iter().take(limit), "json-cosine-fallback");
}

fn print_vector_rows<I>(rows: I, vector_runtime: &str)
where
    I: IntoIterator<Item = (f32, f32, f32, f64, Vec<String>)>,
{
    let mut out = String::from("[");
    for (index, (score, vector_score, text_quality, distance, row)) in rows.into_iter().enumerate()
    {
        if row.len() < 7 {
            continue;
        }
        if index > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            "{{\"chunk_id\":{},\"path\":{},\"heading_path\":{},\"excerpt\":{},\"score\":{:.6},\"vector_score\":{:.6},\"vector_distance\":{:.6},\"text_quality_score\":{:.3},\"vector_runtime\":{},\"content_hash\":{},\"modified_unix\":{},\"human_relevance_score\":{}}}",
            json_string(&row[0]),
            json_string(&row[1]),
            json_string(&row[2]),
            json_string(&row[3]),
            score,
            vector_score,
            distance,
            text_quality,
            json_string(vector_runtime),
            json_string(&row[4]),
            row[5],
            row[6]
        ));
    }
    out.push(']');
    println!("{out}");
}

fn hybrid_search_command(args: Vec<String>) {
    let (query, rest) = split_query(args);
    let options = CliOptions::parse(rest);
    let db = require_db(&options);
    let limit = options.limit.unwrap_or(10) as usize;
    let candidate_limit = (limit * 12).max(25);
    let provider = EmbeddingProvider::from_model_option(options.embedding_model.as_deref())
        .unwrap_or_else(|error| fail(&error));
    let fts = sqlite_fts_query(&query);
    if fts.is_empty() {
        println!("[]");
        return;
    }
    let query_batch = provider
        .embed_batch(&[query.clone()], EmbeddingKind::Query)
        .unwrap_or_else(|error| fail(&format!("query embedding failed: {error}")));
    let query_embedding = query_batch
        .embeddings
        .first()
        .map(|embedding| embedding_from_json(embedding))
        .unwrap_or_default();
    let sql = format!(
        "SELECT s.id, n.path, s.heading_path, substr(s.text, 1, 240), s.content_hash, n.modified_unix, s.human_relevance_score, e.embedding_json, s.text, bm25(sections_fts) * -1.0 AS fts_score FROM sections_fts JOIN sections s ON s.id = sections_fts.id JOIN notes n ON n.id = s.note_id LEFT JOIN embeddings e ON e.chunk_id = s.id AND e.model = {} WHERE sections_fts MATCH {} ORDER BY fts_score DESC LIMIT {};",
        sql_literal(provider.model_name()), sql_literal(&fts), candidate_limit
    );
    let mut scored = sqlite_table(&db, &sql)
        .into_iter()
        .filter_map(|row| {
            if row.len() < 10 {
                return None;
            }
            let fts_score = row[9].parse::<f32>().unwrap_or(0.0).max(0.0);
            let vector_score = if row[7].is_empty() {
                0.0
            } else {
                cosine_similarity(&query_embedding, &embedding_from_json(&row[7]))
            };
            let text_quality = retrieval_text_quality_score(&row[8]);
            let human = row[6].parse::<f32>().unwrap_or(0.5).clamp(0.0, 1.0);
            let score = (fts_score * 0.65 + vector_score * 0.25 + human * 0.10) * text_quality;
            Some((score, fts_score, vector_score, text_quality, row))
        })
        .collect::<Vec<_>>();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let mut out = String::from("[");
    for (index, (score, fts_score, vector_score, text_quality, row)) in
        scored.into_iter().take(limit).enumerate()
    {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            "{{\"chunk_id\":{},\"path\":{},\"heading_path\":{},\"excerpt\":{},\"score\":{:.6},\"fts_score\":{:.6},\"vector_score\":{:.6},\"text_quality_score\":{:.3},\"ranking_runtime\":\"fts-vector-quality\",\"content_hash\":{},\"modified_unix\":{},\"human_relevance_score\":{}}}",
            json_string(&row[0]), json_string(&row[1]), json_string(&row[2]), json_string(&row[3]), score, fts_score, vector_score, text_quality, json_string(&row[4]), row[5], row[6]
        ));
    }
    out.push(']');
    println!("{out}");
}

fn context_command(args: Vec<String>) {
    let (query, rest) = split_query(args);
    let options = CliOptions::parse(rest);
    let db = require_db(&options);
    let sql = if is_duckdb_path(&db) {
        let like_query = format!("%{}%", query);
        format!(
            "SELECT s.id AS chunk_id, n.path, s.heading_path, substr(s.text, 1, 700) AS excerpt, s.content_hash, n.modified_unix, s.human_relevance_score \
             FROM sections s JOIN notes n ON n.id = s.note_id \
             WHERE s.text LIKE {} OR n.path LIKE {} OR s.heading_path LIKE {} LIMIT {};",
            sql_literal(&like_query), sql_literal(&like_query), sql_literal(&like_query), options.limit.unwrap_or(8)
        )
    } else {
        sqlite_fts_search_sql(&query, options.limit.unwrap_or(8), 700)
    };
    print_query_json(&db, &sql);
}

fn serve_command(args: Vec<String>) {
    let options = CliOptions::parse(args);
    if !options.mcp {
        println!("VaultLayer serve currently supports --mcp only");
        return;
    }
    if options.list_tools {
        println!(
            "[{{\"name\":\"vault_search\",\"description\":\"Search indexed vault chunks with citations\"}},{{\"name\":\"vault_get_note\",\"description\":\"Return one bounded note with provenance\"}},{{\"name\":\"vault_related\",\"description\":\"Return WikiLink/backlink related notes\"}}]"
        );
        return;
    }
    let Some(call) = options.call.as_deref() else {
        println!("{{\"mcp\":\"vault-layer\",\"status\":\"ready\",\"tools\":[\"vault_search\",\"vault_get_note\",\"vault_related\"]}}");
        return;
    };
    let db = require_db(&options);
    let query = options.query.clone().unwrap_or_default();
    match call {
        "vault_search" => {
            let limit = options.limit.unwrap_or(10);
            let like_query = format!("%{}%", query);
            let sql = format!("SELECT s.id AS chunk_id, n.path, s.heading_path, substr(s.text, 1, 240) AS excerpt, CAST(0.0 AS DOUBLE) AS score, s.content_hash, n.modified_unix, s.human_relevance_score FROM sections s JOIN notes n ON n.id = s.note_id WHERE s.text LIKE {} OR n.path LIKE {} OR s.heading_path LIKE {} ORDER BY s.human_relevance_score DESC, n.modified_unix DESC LIMIT {};", sql_literal(&like_query), sql_literal(&like_query), sql_literal(&like_query), limit);
            print_query_json(&db, &sql);
        }
        "vault_get_note" => {
            let like = format!("%{}%", query);
            let sql = format!("SELECT n.id, n.path, n.title, n.modified_unix, n.content_hash, n.human_relevance_score, substr(group_concat(s.heading_path || ': ' || s.text, char(10)), 1, 4000) AS bounded_content FROM notes n LEFT JOIN sections s ON s.note_id = n.id WHERE n.id = {} OR n.path = {} OR n.path LIKE {} GROUP BY n.id ORDER BY n.path LIMIT 1;", sql_literal(&query), sql_literal(&query), sql_literal(&like));
            print_query_json(&db, &sql);
        }
        "vault_related" => {
            let like = format!("%{}%", query);
            let sql = format!("WITH base AS (SELECT id, path FROM notes WHERE id = {} OR path = {} OR path LIKE {} LIMIT 1), outgoing AS (SELECT l.target AS relation, l.raw AS evidence FROM links l JOIN base b ON b.id = l.source_note_id), incoming AS (SELECT n.path AS relation, l.raw AS evidence FROM links l JOIN notes n ON n.id = l.source_note_id JOIN base b ON l.target = replace(b.path, '.md', '')) SELECT 'outgoing' AS kind, relation, evidence FROM outgoing UNION ALL SELECT 'incoming' AS kind, relation, evidence FROM incoming LIMIT 25;", sql_literal(&query), sql_literal(&query), sql_literal(&like));
            print_query_json(&db, &sql);
        }
        other => fail(&format!("unknown MCP tool: {other}")),
    }
}

enum EmbeddingKind {
    Passage,
    Query,
}

struct EmbeddingBatch {
    model: String,
    dimensions: usize,
    cache_dir: Option<String>,
    embeddings: Vec<String>,
}

enum EmbeddingProvider {
    Deterministic,
    FastEmbedMiniLm,
}

impl EmbeddingProvider {
    fn from_model_option(value: Option<&str>) -> Result<Self, String> {
        match value
            .unwrap_or("deterministic-v0")
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "" | "deterministic" | "deterministic-v0" => Ok(Self::Deterministic),
            "fastembed" | "fastembed-mini-lm" | "all-minilm-l6-v2" => Ok(Self::FastEmbedMiniLm),
            other => Err(format!(
                "unknown embedding model '{other}'; supported: deterministic-v0, fastembed-mini-lm"
            )),
        }
    }

    fn model_name(&self) -> &'static str {
        match self {
            Self::Deterministic => "deterministic-v0",
            Self::FastEmbedMiniLm => "fastembed:sentence-transformers/all-MiniLM-L6-v2",
        }
    }

    fn embed_batch(&self, texts: &[String], kind: EmbeddingKind) -> Result<EmbeddingBatch, String> {
        match self {
            Self::Deterministic => Ok(EmbeddingBatch {
                model: self.model_name().to_string(),
                dimensions: 8,
                cache_dir: None,
                embeddings: texts
                    .iter()
                    .map(|text| embedding_to_json(&deterministic_embedding(text, 8)))
                    .collect(),
            }),
            Self::FastEmbedMiniLm => run_fastembed_helper(texts, kind),
        }
    }
}

fn fastembed_cache_dir() -> PathBuf {
    match env::var_os("VAULT_LAYER_FASTEMBED_CACHE_DIR") {
        Some(value) if !value.is_empty() => PathBuf::from(value),
        _ => default_state_dir()
            .unwrap_or_else(|_| PathBuf::from(".vault-layer-state"))
            .join("models/fastembed"),
    }
}

fn fastembed_helper_path() -> PathBuf {
    match env::var_os("VAULT_LAYER_FASTEMBED_HELPER") {
        Some(value) if !value.is_empty() => PathBuf::from(value),
        _ => PathBuf::from("scripts/fastembed_embed.py"),
    }
}

fn run_fastembed_helper(texts: &[String], kind: EmbeddingKind) -> Result<EmbeddingBatch, String> {
    let cache_dir = fastembed_cache_dir();
    fs::create_dir_all(&cache_dir).map_err(|error| {
        format!(
            "create fastembed cache dir {}: {error}",
            cache_dir.display()
        )
    })?;

    let input_path = env::temp_dir().join(format!(
        "vault-layer-fastembed-{}-{}.json",
        std::process::id(),
        texts.len()
    ));
    let mut input = format!(
        "{{\"kind\":{},\"cache_dir\":{},\"texts\":[",
        json_string(match kind {
            EmbeddingKind::Passage => "passage",
            EmbeddingKind::Query => "query",
        }),
        json_string(&cache_dir.display().to_string())
    );
    for (index, text) in texts.iter().enumerate() {
        if index > 0 {
            input.push(',');
        }
        input.push_str(&json_string(text));
    }
    input.push_str("]}");
    fs::write(&input_path, input).map_err(|error| format!("write fastembed input: {error}"))?;

    let python = env::var("VAULT_LAYER_FASTEMBED_PYTHON").unwrap_or_else(|_| "python3".to_string());
    let output = Command::new(python)
        .arg(fastembed_helper_path())
        .arg(&input_path)
        .output()
        .map_err(|error| format!("start fastembed helper: {error}"));
    let _ = fs::remove_file(&input_path);
    let output = output?;
    if !output.status.success() {
        return Err(format!(
            "fastembed helper failed: {}{}",
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout)
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut lines = stdout.lines();
    let metadata = lines
        .next()
        .ok_or_else(|| "fastembed helper returned no metadata".to_string())?;
    if !metadata.contains("fastembed:sentence-transformers/all-MiniLM-L6-v2") {
        return Err(format!("unexpected fastembed metadata: {metadata}"));
    }
    let embeddings = lines.map(ToString::to_string).collect::<Vec<_>>();
    if embeddings.len() != texts.len() {
        return Err(format!(
            "fastembed helper returned {} vectors for {} texts",
            embeddings.len(),
            texts.len()
        ));
    }
    let dimensions = embeddings
        .first()
        .map(|embedding| embedding_from_json(embedding).len())
        .unwrap_or(0);
    if dimensions != 384 {
        return Err(format!("unexpected fastembed dimensions: {dimensions}"));
    }
    Ok(EmbeddingBatch {
        model: "fastembed:sentence-transformers/all-MiniLM-L6-v2".to_string(),
        dimensions,
        cache_dir: Some(cache_dir.display().to_string()),
        embeddings,
    })
}

#[derive(Default)]
struct CliOptions {
    db: Option<PathBuf>,
    limit: Option<u32>,
    query: Option<String>,
    call: Option<String>,
    mcp: bool,
    list_tools: bool,
    remote_sync: bool,
    force: bool,
    embedding_model: Option<String>,
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
                "--mcp" => options.mcp = true,
                "--list-tools" => options.list_tools = true,
                "--remote-sync" => options.remote_sync = true,
                "--force" => options.force = true,
                "--query" => options.query = iter.next(),
                "--call" => options.call = iter.next(),
                "--model" => options.embedding_model = iter.next(),
                _ if arg.starts_with("--db=") => {
                    options.db = Some(PathBuf::from(arg.trim_start_matches("--db=")))
                }
                _ if arg.starts_with("--limit=") => {
                    options.limit = arg.trim_start_matches("--limit=").parse().ok()
                }
                _ if arg.starts_with("--query=") => {
                    options.query = Some(arg.trim_start_matches("--query=").to_string())
                }
                _ if arg.starts_with("--call=") => {
                    options.call = Some(arg.trim_start_matches("--call=").to_string())
                }
                _ if arg.starts_with("--model=") => {
                    options.embedding_model = Some(arg.trim_start_matches("--model=").to_string())
                }
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

fn is_duckdb_path(db: &Path) -> bool {
    db.extension().and_then(|value| value.to_str()) == Some("duckdb")
}

fn sqlite_fts_search_sql(query: &str, limit: u32, excerpt_len: usize) -> String {
    let fts = sqlite_fts_query(query);
    if fts.is_empty() {
        return "SELECT s.id AS chunk_id, n.path, s.heading_path, substr(s.text, 1, 240) AS excerpt, CAST(0.0 AS DOUBLE) AS score, s.content_hash, n.modified_unix, s.human_relevance_score FROM sections s JOIN notes n ON n.id = s.note_id WHERE 0 LIMIT 0;".to_string();
    }
    format!(
        "SELECT s.id AS chunk_id, n.path, s.heading_path, substr(s.text, 1, {excerpt_len}) AS excerpt, bm25(sections_fts) * -1.0 AS score, s.content_hash, n.modified_unix, s.human_relevance_score \
         FROM sections_fts JOIN sections s ON s.id = sections_fts.id JOIN notes n ON n.id = s.note_id \
         WHERE sections_fts MATCH {} \
         ORDER BY score DESC, s.human_relevance_score DESC, n.modified_unix DESC LIMIT {limit};",
        sql_literal(&fts)
    )
}

fn sqlite_fts_query(query: &str) -> String {
    query
        .split_whitespace()
        .map(|part| part.trim_matches(|ch: char| !ch.is_alphanumeric() && ch != '_' && ch != '-'))
        .filter(|part| !part.is_empty())
        .map(|part| format!("\"{}\"", part.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" AND ")
}

fn print_query_json(db: &PathBuf, sql: &str) {
    if is_duckdb_path(db) {
        print_duckdb_json(db, sql);
    } else {
        print_sqlite_json(db, sql);
    }
}

fn print_duckdb_json(db: &PathBuf, sql: &str) {
    let conn = duckdb::Connection::open(db)
        .unwrap_or_else(|error| fail(&format!("duckdb query failed to start: {error}")));
    let mut stmt = conn
        .prepare(sql)
        .unwrap_or_else(|error| fail(&format!("duckdb query prepare failed: {error}")));
    let column_names = duckdb_column_names_for(sql);
    let mut rows = stmt
        .query([])
        .unwrap_or_else(|error| fail(&format!("duckdb query failed: {error}")));
    let mut out = String::from("[");
    let mut row_index = 0usize;
    while let Some(row) = rows
        .next()
        .unwrap_or_else(|error| fail(&format!("duckdb row failed: {error}")))
    {
        if row_index > 0 {
            out.push(',');
        }
        out.push('{');
        for (index, name) in column_names.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            let value = row_to_json_value(row, index);
            out.push_str(&format!("{}:{}", json_string(name), value));
        }
        out.push('}');
        row_index += 1;
    }
    out.push(']');
    println!("{out}");
}

fn duckdb_column_names_for(sql: &str) -> Vec<String> {
    if sql.contains("bounded_content") {
        return [
            "id",
            "path",
            "title",
            "modified_unix",
            "content_hash",
            "human_relevance_score",
            "bounded_content",
        ]
        .iter()
        .map(|value| value.to_string())
        .collect();
    }
    if sql.contains(" AS kind") && sql.contains("relation") {
        return ["kind", "relation", "evidence"]
            .iter()
            .map(|value| value.to_string())
            .collect();
    }
    if sql.contains(" AS chunk_id") || sql.contains("s.id AS chunk_id") {
        return [
            "chunk_id",
            "path",
            "heading_path",
            "excerpt",
            "score",
            "content_hash",
            "modified_unix",
            "human_relevance_score",
        ]
        .iter()
        .map(|value| value.to_string())
        .collect();
    }
    Vec::new()
}

fn row_to_json_value(row: &duckdb::Row<'_>, index: usize) -> String {
    match row.get_ref(index) {
        Ok(duckdb::types::ValueRef::Null) => "null".to_string(),
        Ok(duckdb::types::ValueRef::Boolean(value)) => value.to_string(),
        Ok(duckdb::types::ValueRef::TinyInt(value)) => value.to_string(),
        Ok(duckdb::types::ValueRef::SmallInt(value)) => value.to_string(),
        Ok(duckdb::types::ValueRef::Int(value)) => value.to_string(),
        Ok(duckdb::types::ValueRef::BigInt(value)) => value.to_string(),
        Ok(duckdb::types::ValueRef::HugeInt(value)) => value.to_string(),
        Ok(duckdb::types::ValueRef::UTinyInt(value)) => value.to_string(),
        Ok(duckdb::types::ValueRef::USmallInt(value)) => value.to_string(),
        Ok(duckdb::types::ValueRef::UInt(value)) => value.to_string(),
        Ok(duckdb::types::ValueRef::UBigInt(value)) => value.to_string(),
        Ok(duckdb::types::ValueRef::Float(value)) => value.to_string(),
        Ok(duckdb::types::ValueRef::Double(value)) => value.to_string(),
        Ok(duckdb::types::ValueRef::Text(value)) => json_string(&String::from_utf8_lossy(value)),
        Ok(duckdb::types::ValueRef::Blob(value)) => {
            json_string(&format!("<{} bytes>", value.len()))
        }
        Ok(other) => json_string(&format!("{other:?}")),
        Err(_) => "null".to_string(),
    }
}

fn print_sqlite_json(db: &PathBuf, sql: &str) {
    let output = Command::new("sqlite3")
        .arg("-json")
        .arg(db)
        .arg(sql)
        .output()
        .unwrap_or_else(|error| fail(&format!("sqlite3 failed to start: {error}")));
    if !output.status.success() {
        fail(&format!(
            "sqlite3 query failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    print!("{}", String::from_utf8_lossy(&output.stdout));
}

fn run_sqlite(db: &PathBuf, sql: &str) {
    let mut child = Command::new("sqlite3")
        .arg(db)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| fail(&format!("sqlite3 failed to start: {error}")));
    {
        use std::io::Write;
        let Some(stdin) = child.stdin.as_mut() else {
            fail("sqlite3 stdin unavailable");
        };
        stdin
            .write_all(sql.as_bytes())
            .unwrap_or_else(|error| fail(&format!("sqlite3 stdin write failed: {error}")));
    }
    let output = child
        .wait_with_output()
        .unwrap_or_else(|error| fail(&format!("sqlite3 wait failed: {error}")));
    if !output.status.success() {
        fail(&format!(
            "sqlite3 command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
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
        fail(&format!(
            "sqlite3 table query failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
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
        "VaultLayer\n\nUSAGE:\n    vault-layer <COMMAND> [OPTIONS]\n\nCOMMANDS:\n    init          Initialize config for an external Markdown/Obsidian vault\n    index         Build or refresh the local shadow index outside the repo\n    search        Search indexed vault chunks and return cited JSON results\n    get-note      Return one bounded note with provenance JSON\n    related       Return WikiLink/backlink related notes as JSON\n    embed         Fill selected embedding model rows and native sqlite-vec rows\n    vector-search Search embeddings, preferring native sqlite-vec when available\n    hybrid-search FTS candidates reranked with vector, relevance, and quality\n    context       Build an agent-ready cited context pack\n    serve         Serve MCP interfaces over the local shadow DB\n    backend-info  Report SQLite/Turso/libSQL backend and vector capability mode\n    doctor        Verify local pilot prerequisites without writing to the vault\n    sqlite-vec-info Smoke native sqlite-vec availability via the scoped Rust adapter\n    sync-turso    Write the scanned vault index to Turso/libSQL via HTTPS pipeline\n\nOPTIONS:\n    --state-dir <PATH>    Runtime state directory; default: ~/{DEFAULT_STATE_SUBDIR}\n    --db <PATH>           Shadow DB path for retrieval commands\n    --remote-sync         With TURSO_DATABASE_URL, index writes to Turso/libSQL\n    --force               Rebuild index DB even when existing note count matches\n    --limit <N>           Limit indexed notes/results for smoke runs\n    --model <NAME>        Embedding model: deterministic-v0 or fastembed-mini-lm\n    --json                JSON output (retrieval commands already emit JSON)\n\nSAFETY:\n    Vault files are read-only by default. DB/index/vector artifacts must live outside the repo."
    );
}
