use anyhow::Result;
use rusqlite::Connection;
use std::collections::{HashMap, HashSet};

use crate::db;

pub fn run(conn: &Connection, experiment: &str, format: &str) -> Result<()> {
    let exp_id = db::resolve_experiment_id(conn, experiment)?;

    let (name, status, description): (String, String, Option<String>) = conn.query_row(
        "SELECT name, status, description FROM experiments WHERE id = ?1",
        [&exp_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;

    // Gather variables
    let mut var_stmt = conn.prepare(
        "SELECT name, role, val_list FROM variables WHERE exp_id = ?1 ORDER BY role, name",
    )?;
    let vars: Vec<(String, String, Option<String>)> = var_stmt
        .query_map([&exp_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .collect::<Result<_, _>>()?;

    // Gather runs
    let mut run_stmt = conn.prepare(
        "SELECT id, status, output FROM runs WHERE exp_id = ?1 ORDER BY started_at",
    )?;
    let runs: Vec<(String, String, Option<String>)> = run_stmt
        .query_map([&exp_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .collect::<Result<_, _>>()?;

    // Gather run variables for completed runs
    let mut run_var_combos: Vec<HashMap<String, String>> = Vec::new();
    let mut output_keys: HashSet<String> = HashSet::new();

    for (run_id, run_status, output) in &runs {
        let mut rv_stmt = conn.prepare(
            "SELECT var_name, value FROM run_variables WHERE run_id = ?1",
        )?;
        let var_map: HashMap<String, String> = rv_stmt
            .query_map([run_id], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<_, _>>()?;

        if run_status == "completed" || run_status == "running" {
            run_var_combos.push(var_map);
        }

        if let Some(json) = output {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(json) {
                if let Some(obj) = val.as_object() {
                    for key in obj.keys() {
                        output_keys.insert(key.clone());
                    }
                }
            }
        }
    }

    // Compute remaining combinations
    let independent_vars: Vec<(String, Vec<String>)> = vars
        .iter()
        .filter(|(_, role, _)| role == "independent")
        .map(|(name, _, values)| {
            let vals: Vec<String> = values
                .as_ref()
                .map(|v| v.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_default();
            (name.clone(), vals)
        })
        .collect();

    let all_combos = cartesian_product(&independent_vars);
    let remaining: Vec<&HashMap<String, String>> = all_combos
        .iter()
        .filter(|combo| {
            !run_var_combos.iter().any(|done| {
                combo.iter().all(|(k, v)| done.get(k).map_or(false, |dv| dv == v))
            })
        })
        .collect();

    let total_runs = runs.len();
    let completed = runs.iter().filter(|(_, s, _)| s == "completed").count();

    if format == "json" {
        print_json(&name, &exp_id, &status, &description, &vars, total_runs, completed, &output_keys, &remaining);
        return Ok(());
    }

    // Text output
    println!("Experiment: {name} ({exp_id})");
    println!("Status: {status} ({completed}/{} runs completed)", all_combos.len().max(total_runs));
    if let Some(desc) = &description {
        println!("Description: {desc}");
    }

    let controls: Vec<&(String, String, Option<String>)> = vars.iter().filter(|(_, r, _)| r == "control").collect();
    if !controls.is_empty() {
        println!("\nControls:");
        for (name, _, values) in &controls {
            println!("  {name} = {}", values.as_deref().unwrap_or(""));
        }
    }

    let indeps: Vec<&(String, String, Option<String>)> = vars.iter().filter(|(_, r, _)| r == "independent").collect();
    if !indeps.is_empty() {
        println!("\nIndependent variables:");
        for (name, _, values) in &indeps {
            println!("  {name} = [{}]", values.as_deref().unwrap_or(""));
        }
    }

    if !output_keys.is_empty() {
        let mut keys: Vec<&String> = output_keys.iter().collect();
        keys.sort();
        println!("\nOutput keys (from completed runs):");
        println!("  {}", keys.iter().map(|k| k.as_str()).collect::<Vec<_>>().join(", "));
    }

    if !remaining.is_empty() {
        println!("\nRemaining combinations ({}):", remaining.len());
        for combo in &remaining {
            let flags: Vec<String> = combo.iter().map(|(k, v)| format!("--{k}=\"{v}\"")).collect();
            println!("  {}", flags.join(" "));
        }
        println!("\nTo start the next run:");
        if let Some(first) = remaining.first() {
            let flags: Vec<String> = first.iter().map(|(k, v)| format!("--{k}=\"{v}\"")).collect();
            println!("  RUN=$(exp run start {name} {})", flags.join(" "));
            println!("  <your command> | exp run record \"$RUN\" --output -");
        }
    } else if total_runs > 0 {
        println!("\nAll combinations have been run.");
    }

    Ok(())
}

fn cartesian_product(vars: &[(String, Vec<String>)]) -> Vec<HashMap<String, String>> {
    if vars.is_empty() {
        return vec![HashMap::new()];
    }

    let (name, values) = &vars[0];
    let rest = cartesian_product(&vars[1..]);

    let mut result = Vec::new();
    for val in values {
        for combo in &rest {
            let mut new_combo = combo.clone();
            new_combo.insert(name.clone(), val.clone());
            result.push(new_combo);
        }
    }
    result
}

fn print_json(
    name: &str,
    id: &str,
    status: &str,
    description: &Option<String>,
    vars: &[(String, String, Option<String>)],
    total_runs: usize,
    completed: usize,
    output_keys: &HashSet<String>,
    remaining: &[&HashMap<String, String>],
) {
    let controls: Vec<serde_json::Value> = vars
        .iter()
        .filter(|(_, r, _)| r == "control")
        .map(|(n, _, v)| serde_json::json!({"name": n, "value": v}))
        .collect();

    let independents: Vec<serde_json::Value> = vars
        .iter()
        .filter(|(_, r, _)| r == "independent")
        .map(|(n, _, v)| serde_json::json!({"name": n, "values": v}))
        .collect();

    let remaining_json: Vec<serde_json::Value> = remaining
        .iter()
        .map(|combo| serde_json::json!(combo))
        .collect();

    let mut keys: Vec<&String> = output_keys.iter().collect();
    keys.sort();

    let doc = serde_json::json!({
        "name": name,
        "id": id,
        "status": status,
        "description": description,
        "controls": controls,
        "independents": independents,
        "total_runs": total_runs,
        "completed_runs": completed,
        "output_keys": keys,
        "remaining_combinations": remaining_json,
    });

    println!("{}", serde_json::to_string_pretty(&doc).unwrap());
}
