use anyhow::Result;
use rusqlite::Connection;

use crate::display;

pub fn run(conn: &Connection, status_filter: Option<&str>) -> Result<()> {
    let mut sql = "SELECT id, name, status, created_at FROM experiments".to_string();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = vec![];

    if let Some(status) = status_filter {
        sql.push_str(" WHERE status = ?1");
        params.push(Box::new(status.to_string()));
    }

    sql.push_str(" ORDER BY created_at DESC");

    let mut stmt = conn.prepare(&sql)?;
    let rows: Vec<Vec<String>> = stmt
        .query_map(rusqlite::params_from_iter(&params), |row| {
            Ok(vec![
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ])
        })?
        .collect::<Result<_, _>>()?;

    if rows.is_empty() {
        println!("No experiments found.");
        return Ok(());
    }

    let table = display::build_table(&["id", "name", "status", "created_at"], &rows);
    println!("{table}");
    Ok(())
}
