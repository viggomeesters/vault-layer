//! Small native sqlite-vec adapter for VaultLayer.
//!
//! The main workspace forbids unsafe code. sqlite-vec registration requires the
//! standard SQLite extension registration call through `sqlite3_auto_extension`,
//! so the unsafe boundary is isolated in this crate.

use rusqlite::{ffi::sqlite3_auto_extension, Connection};
use sqlite_vec::sqlite3_vec_init;
use zerocopy::AsBytes;

#[derive(Debug, Clone, PartialEq)]
pub struct SqliteVecSmoke {
    pub version: String,
    pub distance: f64,
    pub dimensions: usize,
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

fn register_sqlite_vec() {
    // SAFETY: sqlite-vec exposes the C SQLite extension entrypoint
    // `sqlite3_vec_init`. Registering it with SQLite's process-wide
    // `sqlite3_auto_extension` is the integration path documented by sqlite-vec
    // for Rust/rusqlite. This crate exists solely to contain that unsafe FFI
    // boundary; callers receive a safe Rust function.
    unsafe {
        sqlite3_auto_extension(Some(std::mem::transmute(sqlite3_vec_init as *const ())));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqlite_vec_smoke_registers_extension_and_queries_distance() {
        let smoke = sqlite_vec_smoke().expect("sqlite-vec smoke");
        assert!(smoke.version.starts_with('v'));
        assert_eq!(smoke.dimensions, 3);
        assert_eq!(smoke.distance, 0.0);
    }
}
