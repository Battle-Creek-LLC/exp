use anyhow::Result;
use rusqlite::Connection;
use std::collections::HashMap;

use crate::db;

pub fn run(conn: &Connection, experiment: &str, shell: &str) -> Result<()> {
    let exp_id = db::resolve_experiment_id(conn, experiment)?;

    let name: String = conn.query_row(
        "SELECT name FROM experiments WHERE id = ?1",
        [&exp_id],
        |row| row.get(0),
    )?;

    // Get independent variables
    let mut var_stmt = conn.prepare(
        "SELECT name, val_list FROM variables WHERE exp_id = ?1 AND role = 'independent' ORDER BY name",
    )?;
    let indep_vars: Vec<(String, Vec<String>)> = var_stmt
        .query_map([&exp_id], |row| {
            let name: String = row.get(0)?;
            let values: Option<String> = row.get(1)?;
            Ok((name, values))
        })?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|(name, values)| {
            let vals: Vec<String> = values
                .map(|v| v.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_default();
            (name, vals)
        })
        .collect();

    // Get completed run variable combos
    let mut run_stmt = conn.prepare(
        "SELECT id FROM runs WHERE exp_id = ?1 AND (status = 'completed' OR status = 'running')",
    )?;
    let run_ids: Vec<String> = run_stmt
        .query_map([&exp_id], |row| row.get(0))?
        .collect::<Result<_, _>>()?;

    let mut done_combos: Vec<HashMap<String, String>> = Vec::new();
    for run_id in &run_ids {
        let mut rv_stmt = conn.prepare(
            "SELECT var_name, value FROM run_variables WHERE run_id = ?1",
        )?;
        let var_map: HashMap<String, String> = rv_stmt
            .query_map([run_id], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<_, _>>()?;
        done_combos.push(var_map);
    }

    let all_combos = cartesian_product(&indep_vars);
    let remaining: Vec<&HashMap<String, String>> = all_combos
        .iter()
        .filter(|combo| {
            !done_combos.iter().any(|done| {
                combo.iter().all(|(k, v)| done.get(k).map_or(false, |dv| dv == v))
            })
        })
        .collect();

    if remaining.is_empty() {
        println!("# All combinations have been run for: {name}");
        return Ok(());
    }

    match shell {
        "bash" | "zsh" => print_bash(&name, &remaining),
        _ => anyhow::bail!("unsupported shell: {shell}. Use bash or zsh."),
    }

    Ok(())
}

fn print_bash(experiment: &str, remaining: &[&HashMap<String, String>]) {
    println!("#!/bin/bash");
    println!("set -euo pipefail");
    println!("# Run plan for: {experiment}");
    println!("# {} runs remaining", remaining.len());
    println!();

    for combo in remaining {
        let mut flags: Vec<String> = combo.iter().map(|(k, v)| format!("--{k}=\"{v}\"")).collect();
        flags.sort();
        let flag_str = flags.join(" ");

        println!("RUN=$(exp run start {experiment} {flag_str})");
        println!("# TODO: replace with your command");
        println!("YOUR_COMMAND | exp run record \"$RUN\" --output -");
        println!();
    }

    println!("exp compare {experiment}");
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
