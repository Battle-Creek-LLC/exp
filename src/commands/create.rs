use anyhow::Result;
use rusqlite::Connection;

use crate::db;

pub fn run(conn: &Connection, name: &str, description: Option<&str>, template: Option<&str>) -> Result<()> {
    let id = db::new_id();
    let now = db::now();

    conn.execute(
        "INSERT INTO experiments (id, name, description, template, status, created_at) VALUES (?1, ?2, ?3, ?4, 'draft', ?5)",
        rusqlite::params![id, name, description, template, now],
    )?;

    if let Some(tmpl) = template {
        apply_template(conn, &id, tmpl)?;
    }

    println!("{id}");
    Ok(())
}

fn apply_template(conn: &Connection, exp_id: &str, template: &str) -> Result<()> {
    let vars = crate::commands::templates::template_variables(template);
    for (role, name, values) in vars {
        let var_id = db::new_id();
        conn.execute(
            "INSERT INTO variables (id, exp_id, name, role, val_list) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![var_id, exp_id, name, role, values],
        )?;
    }
    Ok(())
}
