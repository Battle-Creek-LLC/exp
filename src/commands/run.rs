use anyhow::{Context, Result};
use rusqlite::Connection;
use std::collections::HashMap;
use std::io::Read;

use crate::db;
use crate::display;

pub fn start(conn: &Connection, experiment: &str, vars: &[(String, String)]) -> Result<()> {
    let exp_id = db::resolve_experiment_id(conn, experiment)?;

    // Update experiment status to running if it's still draft
    conn.execute(
        "UPDATE experiments SET status = 'running' WHERE id = ?1 AND status = 'draft'",
        [&exp_id],
    )?;

    let run_id = db::new_id();
    let now = db::now();

    conn.execute(
        "INSERT INTO runs (id, exp_id, status, started_at) VALUES (?1, ?2, 'running', ?3)",
        rusqlite::params![run_id, exp_id, now],
    )?;

    for (name, value) in vars {
        conn.execute(
            "INSERT INTO run_variables (run_id, var_name, value) VALUES (?1, ?2, ?3)",
            rusqlite::params![run_id, name, value],
        )?;
    }

    // Print only the run ID so it can be captured with $()
    println!("{run_id}");
    Ok(())
}

pub fn record(conn: &Connection, run_id: &str, output_source: &str) -> Result<()> {
    let json_str = if output_source == "-" {
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)?;
        buf
    } else if output_source.starts_with('{') || output_source.starts_with('[') {
        output_source.to_string()
    } else {
        std::fs::read_to_string(output_source)
            .with_context(|| format!("reading output file: {output_source}"))?
    };

    // Validate JSON
    let new_value: serde_json::Value =
        serde_json::from_str(&json_str).with_context(|| "output must be valid JSON")?;

    // Merge with existing output if any
    let existing: Option<String> = conn.query_row(
        "SELECT output FROM runs WHERE id = ?1",
        [run_id],
        |row| row.get(0),
    )?;

    let merged = if let Some(existing_str) = existing {
        let mut existing_val: serde_json::Value = serde_json::from_str(&existing_str)?;
        if let (Some(existing_obj), Some(new_obj)) = (existing_val.as_object_mut(), new_value.as_object()) {
            for (k, v) in new_obj {
                existing_obj.insert(k.clone(), v.clone());
            }
        }
        serde_json::to_string(&existing_val)?
    } else {
        serde_json::to_string(&new_value)?
    };

    let now = db::now();
    conn.execute(
        "UPDATE runs SET output = ?1, status = 'completed', finished_at = ?2 WHERE id = ?3",
        rusqlite::params![merged, now, run_id],
    )?;

    Ok(())
}

pub fn fail(conn: &Connection, run_id: &str, reason: Option<&str>) -> Result<()> {
    let now = db::now();

    // Store reason as JSON output if provided
    if let Some(reason) = reason {
        let output = serde_json::json!({"error": reason}).to_string();
        conn.execute(
            "UPDATE runs SET status = 'failed', finished_at = ?1, output = ?2 WHERE id = ?3",
            rusqlite::params![now, output, run_id],
        )?;
    } else {
        conn.execute(
            "UPDATE runs SET status = 'failed', finished_at = ?1 WHERE id = ?2",
            rusqlite::params![now, run_id],
        )?;
    }

    Ok(())
}

pub fn comment(conn: &Connection, run_id: &str, body: &str) -> Result<()> {
    let id = db::new_id();
    let now = db::now();

    conn.execute(
        "INSERT INTO comments (id, run_id, body, added_at) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![id, run_id, body, now],
    )?;

    Ok(())
}

pub fn artifact(conn: &Connection, run_id: &str, file_path: &str) -> Result<()> {
    let content = std::fs::read(file_path)
        .with_context(|| format!("reading artifact: {file_path}"))?;

    let name = std::path::Path::new(file_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| file_path.to_string());

    let id = db::new_id();
    let now = db::now();

    conn.execute(
        "INSERT INTO artifacts (id, run_id, name, content, added_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![id, run_id, name, content, now],
    )?;

    Ok(())
}

pub fn list(conn: &Connection, experiment: &str) -> Result<()> {
    let exp_id = db::resolve_experiment_id(conn, experiment)?;

    let mut stmt = conn.prepare(
        "SELECT id, status, started_at FROM runs WHERE exp_id = ?1 ORDER BY started_at",
    )?;
    let runs: Vec<(String, String, Option<String>)> = stmt
        .query_map([&exp_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?
        .collect::<Result<_, _>>()?;

    if runs.is_empty() {
        println!("No runs.");
        return Ok(());
    }

    // Gather variables for each run
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut all_var_names: Vec<String> = Vec::new();

    // Collect all variable names first
    for (run_id, _, _) in &runs {
        let mut var_stmt = conn.prepare(
            "SELECT var_name FROM run_variables WHERE run_id = ?1 ORDER BY var_name",
        )?;
        let names: Vec<String> = var_stmt
            .query_map([run_id], |row| row.get(0))?
            .collect::<Result<_, _>>()?;
        for name in names {
            if !all_var_names.contains(&name) {
                all_var_names.push(name);
            }
        }
    }

    for (run_id, status, started_at) in &runs {
        let mut var_stmt = conn.prepare(
            "SELECT var_name, value FROM run_variables WHERE run_id = ?1",
        )?;
        let var_map: HashMap<String, String> = var_stmt
            .query_map([run_id], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<_, _>>()?;

        let mut row = vec![
            run_id.clone(),
            status.clone(),
            started_at.clone().unwrap_or_default(),
        ];
        for var_name in &all_var_names {
            row.push(var_map.get(var_name).cloned().unwrap_or_default());
        }
        rows.push(row);
    }

    let mut headers: Vec<&str> = vec!["run", "status", "started_at"];
    let var_name_refs: Vec<&str> = all_var_names.iter().map(|s| s.as_str()).collect();
    headers.extend(var_name_refs.iter());

    let table = display::build_table(&headers, &rows);
    println!("{table}");
    Ok(())
}

pub fn show(conn: &Connection, run_id: &str) -> Result<()> {
    let (exp_id, status, started_at, finished_at, output): (
        String, String, Option<String>, Option<String>, Option<String>,
    ) = conn.query_row(
        "SELECT exp_id, status, started_at, finished_at, output FROM runs WHERE id = ?1",
        [run_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
    ).with_context(|| format!("run not found: {run_id}"))?;

    let exp_name: String = conn.query_row(
        "SELECT name FROM experiments WHERE id = ?1",
        [&exp_id],
        |row| row.get(0),
    )?;

    println!("Run: {run_id}");
    println!("Experiment: {exp_name}");
    println!("Status: {status}");
    if let Some(s) = started_at {
        println!("Started: {s}");
    }
    if let Some(f) = finished_at {
        println!("Finished: {f}");
    }

    // Variables
    let mut stmt = conn.prepare(
        "SELECT var_name, value FROM run_variables WHERE run_id = ?1 ORDER BY var_name",
    )?;
    let vars: Vec<(String, String)> = stmt
        .query_map([run_id], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<_, _>>()?;

    if !vars.is_empty() {
        println!("\nVariables:");
        for (name, value) in &vars {
            println!("  {name} = {value}");
        }
    }

    // Output
    if let Some(output) = output {
        let pretty: serde_json::Value = serde_json::from_str(&output)?;
        println!("\nOutput:");
        println!("{}", serde_json::to_string_pretty(&pretty)?);
    }

    // Artifacts
    let mut stmt = conn.prepare(
        "SELECT name, added_at, length(content) FROM artifacts WHERE run_id = ?1",
    )?;
    let artifacts: Vec<(String, String, i64)> = stmt
        .query_map([run_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .collect::<Result<_, _>>()?;

    if !artifacts.is_empty() {
        println!("\nArtifacts:");
        for (name, added_at, size) in &artifacts {
            println!("  {name} ({size} bytes, {added_at})");
        }
    }

    // Comments
    let mut stmt = conn.prepare(
        "SELECT body, added_at FROM comments WHERE run_id = ?1 ORDER BY added_at",
    )?;
    let comments: Vec<(String, String)> = stmt
        .query_map([run_id], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<_, _>>()?;

    if !comments.is_empty() {
        println!("\nComments:");
        for (body, added_at) in &comments {
            println!("  [{added_at}] {body}");
        }
    }

    Ok(())
}
