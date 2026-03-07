use anyhow::Result;
use rusqlite::Connection;

use crate::db;
use crate::display;

pub fn set(
    conn: &Connection,
    experiment: &str,
    controls: &[(String, String)],
    independents: &[(String, String)],
) -> Result<()> {
    let exp_id = db::resolve_experiment_id(conn, experiment)?;

    for (name, value) in controls {
        upsert_variable(conn, &exp_id, name, "control", value)?;
    }
    for (name, values) in independents {
        upsert_variable(conn, &exp_id, name, "independent", values)?;
    }

    Ok(())
}

fn upsert_variable(conn: &Connection, exp_id: &str, name: &str, role: &str, values: &str) -> Result<()> {
    let existing: Option<String> = conn
        .query_row(
            "SELECT id FROM variables WHERE exp_id = ?1 AND name = ?2",
            rusqlite::params![exp_id, name],
            |row| row.get(0),
        )
        .ok();

    if let Some(id) = existing {
        conn.execute(
            "UPDATE variables SET role = ?1, val_list = ?2 WHERE id = ?3",
            rusqlite::params![role, values, id],
        )?;
    } else {
        let id = db::new_id();
        conn.execute(
            "INSERT INTO variables (id, exp_id, name, role, val_list) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![id, exp_id, name, role, values],
        )?;
    }

    Ok(())
}

pub fn list(conn: &Connection, experiment: &str) -> Result<()> {
    let exp_id = db::resolve_experiment_id(conn, experiment)?;

    let mut stmt = conn.prepare(
        "SELECT name, role, val_list FROM variables WHERE exp_id = ?1 ORDER BY role, name",
    )?;
    let rows: Vec<Vec<String>> = stmt
        .query_map([&exp_id], |row| {
            Ok(vec![
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?.unwrap_or_default(),
            ])
        })?
        .collect::<Result<_, _>>()?;

    if rows.is_empty() {
        println!("No variables defined.");
        return Ok(());
    }

    let table = display::build_table(&["name", "role", "values"], &rows);
    println!("{table}");
    Ok(())
}

pub fn rm(conn: &Connection, experiment: &str, name: &str) -> Result<()> {
    let exp_id = db::resolve_experiment_id(conn, experiment)?;

    let affected = conn.execute(
        "DELETE FROM variables WHERE exp_id = ?1 AND name = ?2",
        rusqlite::params![exp_id, name],
    )?;

    if affected == 0 {
        anyhow::bail!("variable not found: {name}");
    }

    Ok(())
}
