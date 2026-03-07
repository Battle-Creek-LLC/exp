use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::Path;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS experiments (
    id          TEXT PRIMARY KEY,
    name        TEXT UNIQUE NOT NULL,
    description TEXT,
    template    TEXT,
    status      TEXT NOT NULL DEFAULT 'draft',
    created_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS variables (
    id      TEXT PRIMARY KEY,
    exp_id  TEXT NOT NULL REFERENCES experiments(id) ON DELETE CASCADE,
    name    TEXT NOT NULL,
    role    TEXT NOT NULL CHECK (role IN ('control', 'independent')),
    val_list TEXT,
    UNIQUE(exp_id, name)
);

CREATE TABLE IF NOT EXISTS runs (
    id          TEXT PRIMARY KEY,
    exp_id      TEXT NOT NULL REFERENCES experiments(id) ON DELETE CASCADE,
    status      TEXT NOT NULL DEFAULT 'pending',
    started_at  TEXT,
    finished_at TEXT,
    output      TEXT
);

CREATE TABLE IF NOT EXISTS run_variables (
    run_id   TEXT NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    var_name TEXT NOT NULL,
    value    TEXT NOT NULL,
    PRIMARY KEY (run_id, var_name)
);

CREATE TABLE IF NOT EXISTS artifacts (
    id       TEXT PRIMARY KEY,
    run_id   TEXT NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    name     TEXT NOT NULL,
    content  BLOB NOT NULL,
    added_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS comments (
    id        TEXT PRIMARY KEY,
    exp_id    TEXT REFERENCES experiments(id) ON DELETE CASCADE,
    run_id    TEXT REFERENCES runs(id) ON DELETE CASCADE,
    body      TEXT NOT NULL,
    added_at  TEXT NOT NULL
);
"#;

pub fn open(db_path: &Path) -> Result<Connection> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating database directory: {}", parent.display()))?;
    }
    let conn = Connection::open(db_path).with_context(|| "opening database")?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
        .with_context(|| "setting pragmas")?;
    conn.execute_batch(SCHEMA)
        .with_context(|| "initializing schema")?;
    Ok(conn)
}

pub fn resolve_experiment_id(conn: &Connection, name: &str) -> Result<String> {
    conn.query_row(
        "SELECT id FROM experiments WHERE name = ?1 OR id = ?1",
        [name],
        |row| row.get(0),
    )
    .with_context(|| format!("experiment not found: {name}"))
}

pub fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

pub fn new_id() -> String {
    ulid::Ulid::new().to_string()
}
