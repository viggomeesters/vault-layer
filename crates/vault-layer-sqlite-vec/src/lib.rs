//! Small native sqlite-vec adapter for VaultLayer.
//!
//! The main workspace forbids unsafe code. sqlite-vec registration requires the
//! standard SQLite extension registration call through `sqlite3_auto_extension`,
//! so the unsafe boundary is isolated in this crate.

use rusqlite::{ffi::sqlite3_auto_extension, params, Connection};
use sqlite_vec::sqlite3_vec_init;
use std::path::Path;
use zerocopy::AsBytes;

#[derive(Debug, Clone, PartialEq)]
pub struct SqliteVecSmoke {
    pub version: String,
    pub distance: f64,
    pub dimensions: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SqliteVecRefresh {
    pub rows: usize,
    pub dimensions: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SqliteVecHit {
    pub chunk_id: String,
    pub distance: f64,
}

pub fn sqlite_vec_smoke() -> Result<SqliteVecSmoke, String> {
    register_sqlite_vec();
    let db = Connection::open_in_memory().map_err(|error| error.to_string())?;
    let vector: Vec<f32> = vec![0.1, 0.2, 0.3];
    let (version, _embedding): (String, String) = db
        .query_row(
            "select vec_version(), vec_to_json(?)",
            [&vector.as_bytes()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|error| error.to_string())?;

    db.execute(
        "create virtual table vec_smoke using vec0(embedding float[3])",
        [],
    )
    .map_err(|error| error.to_string())?;
    db.execute(
        "insert into vec_smoke(rowid, embedding) values (1, ?)",
        [&vector.as_bytes()],
    )
    .map_err(|error| error.to_string())?;
    let distance: f64 = db
        .query_row(
            "select distance from vec_smoke where embedding match ? order by distance limit 1",
            [&vector.as_bytes()],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;

    Ok(SqliteVecSmoke {
        version,
        distance,
        dimensions: vector.len(),
    })
}

pub fn refresh_vec_embeddings<P: AsRef<Path>>(
    db_path: P,
    model: &str,
    dimensions: usize,
) -> Result<SqliteVecRefresh, String> {
    register_sqlite_vec();
    let mut db = Connection::open(db_path).map_err(|error| error.to_string())?;
    db.execute_batch(&format!(
        "CREATE TABLE IF NOT EXISTS vec_embedding_map(rowid INTEGER PRIMARY KEY, chunk_id TEXT NOT NULL UNIQUE, model TEXT NOT NULL);\
         DROP TABLE IF EXISTS vec_embeddings;\
         CREATE VIRTUAL TABLE vec_embeddings USING vec0(embedding float[{dimensions}]);\
         DELETE FROM vec_embedding_map;"
    ))
    .map_err(|error| error.to_string())?;

    let rows = {
        let mut stmt = db
            .prepare(
                "SELECT chunk_id, embedding_json FROM embeddings WHERE model = ? ORDER BY chunk_id;",
            )
            .map_err(|error| error.to_string())?;
        let iter = stmt
            .query_map([model], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|error| error.to_string())?;
        let mut rows = Vec::new();
        for row in iter {
            rows.push(row.map_err(|error| error.to_string())?);
        }
        rows
    };

    let tx = db.transaction().map_err(|error| error.to_string())?;
    let mut inserted = 0usize;
    for (index, (chunk_id, embedding_json)) in rows.into_iter().enumerate() {
        let embedding = parse_embedding_json(&embedding_json);
        if embedding.len() != dimensions {
            continue;
        }
        let rowid = (index + 1) as i64;
        tx.execute(
            "INSERT INTO vec_embedding_map(rowid, chunk_id, model) VALUES (?, ?, ?);",
            params![rowid, chunk_id, model],
        )
        .map_err(|error| error.to_string())?;
        tx.execute(
            "INSERT INTO vec_embeddings(rowid, embedding) VALUES (?, ?);",
            params![rowid, embedding.as_bytes()],
        )
        .map_err(|error| error.to_string())?;
        inserted += 1;
    }
    tx.commit().map_err(|error| error.to_string())?;

    Ok(SqliteVecRefresh {
        rows: inserted,
        dimensions,
    })
}

pub fn search_vec_embeddings<P: AsRef<Path>>(
    db_path: P,
    query_embedding: &[f32],
    limit: usize,
) -> Result<Vec<SqliteVecHit>, String> {
    register_sqlite_vec();
    let db = Connection::open(db_path).map_err(|error| error.to_string())?;
    let sql = "SELECT m.chunk_id, v.distance FROM vec_embeddings v JOIN vec_embedding_map m ON m.rowid = v.rowid WHERE v.embedding MATCH ? AND k = ? ORDER BY v.distance;";
    let mut stmt = db.prepare(sql).map_err(|error| error.to_string())?;
    let iter = stmt
        .query_map(params![query_embedding.as_bytes(), limit as i64], |row| {
            Ok(SqliteVecHit {
                chunk_id: row.get(0)?,
                distance: row.get(1)?,
            })
        })
        .map_err(|error| error.to_string())?;
    let mut hits = Vec::new();
    for hit in iter {
        hits.push(hit.map_err(|error| error.to_string())?);
    }
    Ok(hits)
}

fn parse_embedding_json(value: &str) -> Vec<f32> {
    value
        .trim_matches(|ch| ch == '[' || ch == ']')
        .split(',')
        .filter_map(|part| part.trim().parse::<f32>().ok())
        .collect()
}

fn register_sqlite_vec() {
    // SAFETY: sqlite-vec exposes the C SQLite extension entrypoint
    // `sqlite3_vec_init`. Registering it with SQLite's process-wide
    // `sqlite3_auto_extension` is the integration path documented by sqlite-vec
    // for Rust/rusqlite. This crate exists solely to contain that unsafe FFI
    // boundary; callers receive safe Rust functions.
    unsafe {
        sqlite3_auto_extension(Some(std::mem::transmute(sqlite3_vec_init as *const ())));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use std::fs;

    #[test]
    fn sqlite_vec_smoke_registers_extension_and_queries_distance() {
        let smoke = sqlite_vec_smoke().expect("sqlite-vec smoke");
        assert!(smoke.version.starts_with('v'));
        assert_eq!(smoke.dimensions, 3);
        assert_eq!(smoke.distance, 0.0);
    }

    #[test]
    fn refreshes_and_searches_vec_embeddings_table() {
        let db_path = std::env::temp_dir().join("vault-layer-sqlite-vec-refresh-test.db");
        let _ = fs::remove_file(&db_path);
        let db = Connection::open(&db_path).expect("open fixture db");
        db.execute_batch(
            "CREATE TABLE embeddings(chunk_id TEXT PRIMARY KEY, model TEXT, dimensions INTEGER, embedding_json TEXT, embedded_at_unix INTEGER);\
             INSERT INTO embeddings VALUES ('a', 'deterministic-v0', 3, '[1,0,0]', 0);\
             INSERT INTO embeddings VALUES ('b', 'deterministic-v0', 3, '[0,1,0]', 0);",
        )
        .expect("fixture embeddings");
        drop(db);

        let refresh = refresh_vec_embeddings(&db_path, "deterministic-v0", 3).expect("refresh vec");
        assert_eq!(refresh.rows, 2);
        let hits = search_vec_embeddings(&db_path, &[1.0, 0.0, 0.0], 1).expect("search vec");
        assert_eq!(hits[0].chunk_id, "a");
        assert_eq!(hits[0].distance, 0.0);
        let _ = fs::remove_file(&db_path);
    }
}
