use anyhow::Result;
use rusqlite::Connection;
use std::io::{self, Write};

use crate::db;

pub fn run(conn: &Connection, experiment: &str, force: bool) -> Result<()> {
    let exp_id = db::resolve_experiment_id(conn, experiment)?;

    if !force {
        print!("Delete experiment '{experiment}' and all its data? [y/N] ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled.");
            return Ok(());
        }
    }

    conn.execute("DELETE FROM experiments WHERE id = ?1", [&exp_id])?;
    println!("Deleted.");
    Ok(())
}
