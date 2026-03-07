use anyhow::Result;
use rusqlite::Connection;

use crate::db;

pub fn add(conn: &Connection, experiment: &str, body: &str) -> Result<()> {
    let exp_id = db::resolve_experiment_id(conn, experiment)?;
    let id = db::new_id();
    let now = db::now();

    conn.execute(
        "INSERT INTO comments (id, exp_id, body, added_at) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![id, exp_id, body, now],
    )?;

    Ok(())
}

pub fn list(conn: &Connection, experiment: &str) -> Result<()> {
    let exp_id = db::resolve_experiment_id(conn, experiment)?;

    let mut stmt = conn.prepare(
        "SELECT body, added_at, run_id FROM comments WHERE exp_id = ?1 OR run_id IN (SELECT id FROM runs WHERE exp_id = ?1) ORDER BY added_at",
    )?;
    let comments: Vec<(String, String, Option<String>)> = stmt
        .query_map([&exp_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .collect::<Result<_, _>>()?;

    if comments.is_empty() {
        println!("No comments.");
        return Ok(());
    }

    for (body, added_at, run_id) in &comments {
        let prefix = if let Some(rid) = run_id {
            format!("[{added_at}] (run {}) ", &rid[..8.min(rid.len())])
        } else {
            format!("[{added_at}] ")
        };
        println!("{prefix}{body}");
    }

    Ok(())
}
