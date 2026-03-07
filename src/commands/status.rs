use anyhow::Result;
use rusqlite::Connection;

use crate::db;

pub fn run(conn: &Connection, experiment: &str) -> Result<()> {
    let exp_id = db::resolve_experiment_id(conn, experiment)?;

    let (name, status, description, created_at): (String, String, Option<String>, String) = conn.query_row(
        "SELECT name, status, description, created_at FROM experiments WHERE id = ?1",
        [&exp_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )?;

    println!("Experiment: {name} ({exp_id})");
    println!("Status: {status}");
    if let Some(desc) = description {
        println!("Description: {desc}");
    }
    println!("Created: {created_at}");

    let total_runs: i64 = conn.query_row(
        "SELECT COUNT(*) FROM runs WHERE exp_id = ?1",
        [&exp_id],
        |row| row.get(0),
    )?;
    let completed_runs: i64 = conn.query_row(
        "SELECT COUNT(*) FROM runs WHERE exp_id = ?1 AND status = 'completed'",
        [&exp_id],
        |row| row.get(0),
    )?;
    let failed_runs: i64 = conn.query_row(
        "SELECT COUNT(*) FROM runs WHERE exp_id = ?1 AND status = 'failed'",
        [&exp_id],
        |row| row.get(0),
    )?;

    println!("\nRuns: {completed_runs} completed, {failed_runs} failed, {total_runs} total");

    Ok(())
}
