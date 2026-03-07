use anyhow::Result;
use rusqlite::Connection;
use std::collections::HashMap;

use crate::db;
use crate::display;

pub fn run(
    conn: &Connection,
    experiment: &str,
    sort_by: Option<&str>,
    descending: bool,
    group_by: Option<&str>,
    where_clauses: &[String],
    cols: Option<&str>,
    format: &str,
) -> Result<()> {
    let exp_id = db::resolve_experiment_id(conn, experiment)?;

    // Fetch all completed runs
    let mut stmt = conn.prepare(
        "SELECT id, output FROM runs WHERE exp_id = ?1 AND status = 'completed' ORDER BY started_at",
    )?;
    let runs: Vec<(String, Option<String>)> = stmt
        .query_map([&exp_id], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<_, _>>()?;

    if runs.is_empty() {
        println!("No completed runs to compare.");
        return Ok(());
    }

    // Collect all variable names and output keys
    let mut var_names: Vec<String> = Vec::new();
    let mut output_keys: Vec<String> = Vec::new();
    let mut run_data: Vec<RunRow> = Vec::new();

    for (run_id, output_json) in &runs {
        let mut var_stmt = conn.prepare(
            "SELECT var_name, value FROM run_variables WHERE run_id = ?1",
        )?;
        let var_map: HashMap<String, String> = var_stmt
            .query_map([run_id], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<_, _>>()?;

        for name in var_map.keys() {
            if !var_names.contains(name) {
                var_names.push(name.clone());
            }
        }

        let output_map: HashMap<String, String> = if let Some(json) = output_json {
            let val: serde_json::Value = serde_json::from_str(json)?;
            if let Some(obj) = val.as_object() {
                obj.iter()
                    .map(|(k, v)| {
                        let s = match v {
                            serde_json::Value::String(s) => s.clone(),
                            other => other.to_string(),
                        };
                        (k.clone(), s)
                    })
                    .collect()
            } else {
                HashMap::new()
            }
        } else {
            HashMap::new()
        };

        for key in output_map.keys() {
            if !output_keys.contains(key) {
                output_keys.push(key.clone());
            }
        }

        run_data.push(RunRow {
            run_id: run_id.clone(),
            vars: var_map,
            outputs: output_map,
        });
    }

    var_names.sort();
    output_keys.sort();

    // Apply where filters
    for clause in where_clauses {
        run_data = apply_filter(run_data, clause);
    }

    if run_data.is_empty() {
        println!("No runs match the filter criteria.");
        return Ok(());
    }

    // Apply sort
    if let Some(sort_key) = sort_by {
        run_data.sort_by(|a, b| {
            let va = a.get_value(sort_key);
            let vb = b.get_value(sort_key);
            let cmp = compare_values(&va, &vb);
            if descending { cmp.reverse() } else { cmp }
        });
    }

    // Determine columns
    let selected_cols: Vec<String> = if let Some(c) = cols {
        c.split(',').map(|s| s.trim().to_string()).collect()
    } else {
        let mut all = vec!["run".to_string()];
        all.extend(var_names.iter().cloned());
        all.extend(output_keys.iter().cloned());
        all
    };

    // Group or flat output
    match format {
        "json" => print_json(&run_data, &var_names, &output_keys),
        "csv" => print_csv(&run_data, &selected_cols, &var_names),
        _ => {
            if let Some(group_key) = group_by {
                print_grouped_table(&run_data, &selected_cols, &var_names, group_key);
            } else {
                print_table(&run_data, &selected_cols, &var_names);
            }
        }
    }

    Ok(())
}

struct RunRow {
    run_id: String,
    vars: HashMap<String, String>,
    outputs: HashMap<String, String>,
}

impl RunRow {
    fn get_value(&self, key: &str) -> String {
        if key == "run" {
            return self.run_id.chars().take(8).collect();
        }
        self.vars
            .get(key)
            .or_else(|| self.outputs.get(key))
            .cloned()
            .unwrap_or_default()
    }
}

fn compare_values(a: &str, b: &str) -> std::cmp::Ordering {
    if let (Ok(fa), Ok(fb)) = (a.parse::<f64>(), b.parse::<f64>()) {
        fa.partial_cmp(&fb).unwrap_or(std::cmp::Ordering::Equal)
    } else {
        a.cmp(b)
    }
}

fn apply_filter(data: Vec<RunRow>, clause: &str) -> Vec<RunRow> {
    if let Some((key, val)) = clause.split_once('=') {
        if let Some(key) = key.strip_suffix('!') {
            // !=
            data.into_iter().filter(|r| r.get_value(key) != val).collect()
        } else {
            data.into_iter().filter(|r| r.get_value(key) == val).collect()
        }
    } else if let Some((key, val)) = clause.split_once('~') {
        data.into_iter().filter(|r| r.get_value(key).contains(val)).collect()
    } else if let Some((key, val)) = clause.split_once('<') {
        let threshold: f64 = val.parse().unwrap_or(f64::MAX);
        data.into_iter()
            .filter(|r| r.get_value(key).parse::<f64>().map_or(false, |v| v < threshold))
            .collect()
    } else if let Some((key, val)) = clause.split_once('>') {
        let threshold: f64 = val.parse().unwrap_or(f64::MIN);
        data.into_iter()
            .filter(|r| r.get_value(key).parse::<f64>().map_or(false, |v| v > threshold))
            .collect()
    } else {
        data
    }
}

fn print_table(data: &[RunRow], cols: &[String], _var_names: &[String]) {
    let headers: Vec<&str> = cols.iter().map(|s| s.as_str()).collect();
    let rows: Vec<Vec<String>> = data.iter().map(|r| {
        cols.iter().map(|c| r.get_value(c)).collect()
    }).collect();

    let table = display::build_table(&headers, &rows);
    println!("{table}");
}

fn print_grouped_table(data: &[RunRow], cols: &[String], _var_names: &[String], group_key: &str) {
    let mut groups: Vec<(String, Vec<&RunRow>)> = Vec::new();
    for row in data {
        let group_val = row.get_value(group_key);
        if let Some(g) = groups.iter_mut().find(|(k, _)| k == &group_val) {
            g.1.push(row);
        } else {
            groups.push((group_val, vec![row]));
        }
    }

    for (group_val, rows) in &groups {
        println!("\n{group_key} = {group_val}");
        let headers: Vec<&str> = cols.iter().filter(|c| c.as_str() != group_key).map(|s| s.as_str()).collect();
        let table_rows: Vec<Vec<String>> = rows.iter().map(|r| {
            cols.iter().filter(|c| c.as_str() != group_key).map(|c| r.get_value(c)).collect()
        }).collect();
        let table = display::build_table(&headers, &table_rows);
        println!("{table}");
    }
}

fn print_json(data: &[RunRow], var_names: &[String], output_keys: &[String]) {
    let json_rows: Vec<serde_json::Value> = data.iter().map(|r| {
        let mut map = serde_json::Map::new();
        map.insert("run".to_string(), serde_json::Value::String(r.run_id.clone()));
        for name in var_names {
            if let Some(v) = r.vars.get(name) {
                map.insert(name.clone(), serde_json::Value::String(v.clone()));
            }
        }
        for key in output_keys {
            if let Some(v) = r.outputs.get(key) {
                if let Ok(n) = v.parse::<f64>() {
                    map.insert(key.clone(), serde_json::json!(n));
                } else {
                    map.insert(key.clone(), serde_json::Value::String(v.clone()));
                }
            }
        }
        serde_json::Value::Object(map)
    }).collect();

    println!("{}", serde_json::to_string_pretty(&json_rows).unwrap());
}

fn print_csv(data: &[RunRow], cols: &[String], _var_names: &[String]) {
    println!("{}", cols.join(","));
    for row in data {
        let values: Vec<String> = cols.iter().map(|c| {
            let v = row.get_value(c);
            if v.contains(',') || v.contains('"') {
                format!("\"{}\"", v.replace('"', "\"\""))
            } else {
                v
            }
        }).collect();
        println!("{}", values.join(","));
    }
}
