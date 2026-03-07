use std::process::Command;

fn exp(args: &[&str], db_path: &str) -> (String, String, bool) {
    let bin = env!("CARGO_BIN_EXE_exp");
    let output = Command::new(bin)
        .args(["--db", db_path])
        .args(args)
        .output()
        .expect("failed to execute exp");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (stdout, stderr, output.status.success())
}

fn exp_stdin(args: &[&str], db_path: &str, stdin: &str) -> (String, String, bool) {
    use std::io::Write;
    let bin = env!("CARGO_BIN_EXE_exp");
    let mut child = Command::new(bin)
        .args(["--db", db_path])
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn exp");
    child.stdin.take().unwrap().write_all(stdin.as_bytes()).unwrap();
    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (stdout, stderr, output.status.success())
}

fn temp_db() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

fn db_path(dir: &tempfile::TempDir) -> String {
    dir.path().join("test.db").to_string_lossy().to_string()
}

#[test]
fn test_create_and_list() {
    let dir = temp_db();
    let db = db_path(&dir);

    let (stdout, _, ok) = exp(&["create", "my-exp", "--description", "test experiment"], &db);
    assert!(ok, "create should succeed");
    assert!(!stdout.trim().is_empty(), "should print experiment id");

    let (stdout, _, ok) = exp(&["list"], &db);
    assert!(ok);
    assert!(stdout.contains("my-exp"));
}

#[test]
fn test_var_set_and_list() {
    let dir = temp_db();
    let db = db_path(&dir);

    exp(&["create", "var-test"], &db);

    let (_, _, ok) = exp(&["var", "set", "var-test", "--control", "model=gpt-4o", "--independent", "strategy=direct,cot,react"], &db);
    assert!(ok, "var set should succeed");

    let (stdout, _, ok) = exp(&["var", "list", "var-test"], &db);
    assert!(ok);
    assert!(stdout.contains("model"));
    assert!(stdout.contains("control"));
    assert!(stdout.contains("strategy"));
    assert!(stdout.contains("independent"));
}

#[test]
fn test_var_rm() {
    let dir = temp_db();
    let db = db_path(&dir);

    exp(&["create", "rm-test"], &db);
    exp(&["var", "set", "rm-test", "--control", "model=gpt-4o"], &db);

    let (_, _, ok) = exp(&["var", "rm", "rm-test", "model"], &db);
    assert!(ok);

    let (stdout, _, _) = exp(&["var", "list", "rm-test"], &db);
    assert!(stdout.contains("No variables"));
}

#[test]
fn test_full_run_lifecycle() {
    let dir = temp_db();
    let db = db_path(&dir);

    exp(&["create", "lifecycle"], &db);
    exp(&["var", "set", "lifecycle", "--control", "model=test", "--independent", "strategy=direct,cot"], &db);

    // Start a run
    let (run_id, _, ok) = exp(&["run", "start", "lifecycle", "--strategy=direct"], &db);
    assert!(ok, "run start should succeed");
    let run_id = run_id.trim();
    assert!(!run_id.is_empty());

    // Record output via stdin
    let (_, _, ok) = exp_stdin(
        &["run", "record", run_id, "--output", "-"],
        &db,
        r#"{"accuracy": 0.82, "tokens": 1240}"#,
    );
    assert!(ok, "run record should succeed");

    // Add comment
    let (_, _, ok) = exp(&["run", "comment", run_id, "baseline run"], &db);
    assert!(ok, "run comment should succeed");

    // Show run
    let (stdout, _, ok) = exp(&["run", "show", run_id], &db);
    assert!(ok);
    assert!(stdout.contains("completed"));
    assert!(stdout.contains("accuracy"));
    assert!(stdout.contains("baseline run"));

    // List runs
    let (stdout, _, ok) = exp(&["run", "list", "lifecycle"], &db);
    assert!(ok);
    assert!(stdout.contains("direct"));
}

#[test]
fn test_run_fail() {
    let dir = temp_db();
    let db = db_path(&dir);

    exp(&["create", "fail-test"], &db);

    let (run_id, _, _) = exp(&["run", "start", "fail-test"], &db);
    let run_id = run_id.trim();

    let (_, _, ok) = exp(&["run", "fail", run_id, "--reason", "OOM at batch 47"], &db);
    assert!(ok);

    let (stdout, _, _) = exp(&["run", "show", run_id], &db);
    assert!(stdout.contains("failed"));
    assert!(stdout.contains("OOM at batch 47"));
}

#[test]
fn test_compare_and_sort() {
    let dir = temp_db();
    let db = db_path(&dir);

    exp(&["create", "compare-test"], &db);
    exp(&["var", "set", "compare-test", "--independent", "strategy=direct,cot,cot+fanout"], &db);

    // Create three runs with different results
    let runs = vec![
        ("direct", r#"{"accuracy": 0.82, "tokens": 1240}"#),
        ("cot", r#"{"accuracy": 0.91, "tokens": 2400}"#),
        ("cot+fanout", r#"{"accuracy": 0.94, "tokens": 4100}"#),
    ];

    for (strategy, output) in &runs {
        let (run_id, _, _) = exp(&["run", "start", "compare-test", &format!("--strategy={strategy}")], &db);
        let run_id = run_id.trim();
        exp_stdin(&["run", "record", run_id, "--output", "-"], &db, output);
    }

    // Compare sorted by accuracy
    let (stdout, _, ok) = exp(&["compare", "compare-test", "--sort-by", "accuracy"], &db);
    assert!(ok);
    assert!(stdout.contains("direct"));
    assert!(stdout.contains("cot"));
    assert!(stdout.contains("cot+fanout"));

    // Verify sort order: accuracy ascending means direct first
    let direct_pos = stdout.find("direct").unwrap();
    let cot_pos = stdout.find("| cot").unwrap();
    let fanout_pos = stdout.find("cot+fanout").unwrap();
    assert!(direct_pos < cot_pos, "direct should come before cot in ascending sort");
    assert!(cot_pos < fanout_pos, "cot should come before cot+fanout");
}

#[test]
fn test_compare_where_filter() {
    let dir = temp_db();
    let db = db_path(&dir);

    exp(&["create", "filter-test"], &db);

    let runs = vec![
        ("direct", r#"{"accuracy": 0.82}"#),
        ("cot+fanout", r#"{"accuracy": 0.94}"#),
    ];

    for (strategy, output) in &runs {
        let (run_id, _, _) = exp(&["run", "start", "filter-test", &format!("--strategy={strategy}")], &db);
        exp_stdin(&["run", "record", run_id.trim(), "--output", "-"], &db, output);
    }

    let (stdout, _, ok) = exp(&["compare", "filter-test", "--where", "strategy~fanout"], &db);
    assert!(ok);
    assert!(stdout.contains("cot+fanout"));
    assert!(!stdout.contains("| direct"), "direct should be filtered out");
}

#[test]
fn test_compare_json_format() {
    let dir = temp_db();
    let db = db_path(&dir);

    exp(&["create", "json-test"], &db);

    let (run_id, _, _) = exp(&["run", "start", "json-test", "--temp=0.7"], &db);
    exp_stdin(&["run", "record", run_id.trim(), "--output", "-"], &db, r#"{"score": 0.9}"#);

    let (stdout, _, ok) = exp(&["compare", "json-test", "--format", "json"], &db);
    assert!(ok);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("should be valid JSON");
    assert!(parsed.is_array());
    assert_eq!(parsed.as_array().unwrap().len(), 1);
}

#[test]
fn test_compare_csv_format() {
    let dir = temp_db();
    let db = db_path(&dir);

    exp(&["create", "csv-test"], &db);

    let (run_id, _, _) = exp(&["run", "start", "csv-test", "--temp=0.7"], &db);
    exp_stdin(&["run", "record", run_id.trim(), "--output", "-"], &db, r#"{"score": 0.9}"#);

    let (stdout, _, ok) = exp(&["compare", "csv-test", "--format", "csv"], &db);
    assert!(ok);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert!(lines.len() >= 2, "should have header + data row");
    assert!(lines[0].contains("run"));
    assert!(lines[0].contains("score"));
}

#[test]
fn test_describe() {
    let dir = temp_db();
    let db = db_path(&dir);

    exp(&["create", "describe-test"], &db);
    exp(&["var", "set", "describe-test", "--control", "model=test", "--independent", "strategy=a,b,c"], &db);

    let (run_id, _, _) = exp(&["run", "start", "describe-test", "--strategy=a"], &db);
    exp_stdin(&["run", "record", run_id.trim(), "--output", "-"], &db, r#"{"score": 1}"#);

    let (stdout, _, ok) = exp(&["describe", "describe-test"], &db);
    assert!(ok);
    assert!(stdout.contains("describe-test"));
    assert!(stdout.contains("Controls:"));
    assert!(stdout.contains("Remaining combinations"));
    assert!(stdout.contains("--strategy="));
}

#[test]
fn test_describe_json() {
    let dir = temp_db();
    let db = db_path(&dir);

    exp(&["create", "desc-json"], &db);
    exp(&["var", "set", "desc-json", "--independent", "x=1,2"], &db);

    let (stdout, _, ok) = exp(&["describe", "desc-json", "--format", "json"], &db);
    assert!(ok);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("should be valid JSON");
    assert_eq!(parsed["name"], "desc-json");
    assert!(parsed["remaining_combinations"].is_array());
}

#[test]
fn test_experiment_comments() {
    let dir = temp_db();
    let db = db_path(&dir);

    exp(&["create", "comment-test"], &db);

    let (_, _, ok) = exp(&["comment", "comment-test", "switching to eval-v2"], &db);
    assert!(ok);

    let (stdout, _, ok) = exp(&["comments", "comment-test"], &db);
    assert!(ok);
    assert!(stdout.contains("switching to eval-v2"));
}

#[test]
fn test_status() {
    let dir = temp_db();
    let db = db_path(&dir);

    exp(&["create", "status-test"], &db);

    let (stdout, _, ok) = exp(&["status", "status-test"], &db);
    assert!(ok);
    assert!(stdout.contains("status-test"));
    assert!(stdout.contains("draft"));
    assert!(stdout.contains("Runs: 0 completed"));
}

#[test]
fn test_delete() {
    let dir = temp_db();
    let db = db_path(&dir);

    exp(&["create", "delete-me"], &db);
    let (_, _, ok) = exp(&["delete", "delete-me", "--force"], &db);
    assert!(ok);

    let (stdout, _, _) = exp(&["list"], &db);
    assert!(!stdout.contains("delete-me"));
}

#[test]
fn test_templates_list() {
    let dir = temp_db();
    let db = db_path(&dir);

    let (stdout, _, ok) = exp(&["templates"], &db);
    assert!(ok);
    assert!(stdout.contains("prompt-ab"));
    assert!(stdout.contains("strategy-sweep"));
    assert!(stdout.contains("param-sweep"));
}

#[test]
fn test_templates_show() {
    let dir = temp_db();
    let db = db_path(&dir);

    let (stdout, _, ok) = exp(&["templates", "show", "strategy-sweep"], &db);
    assert!(ok);
    assert!(stdout.contains("strategy"));
    assert!(stdout.contains("Example:"));
}

#[test]
fn test_create_with_template() {
    let dir = temp_db();
    let db = db_path(&dir);

    let (_, _, ok) = exp(&["create", "from-template", "--template", "param-sweep"], &db);
    assert!(ok);

    let (stdout, _, ok) = exp(&["var", "list", "from-template"], &db);
    assert!(ok);
    assert!(stdout.contains("temperature"));
    assert!(stdout.contains("independent"));
}

#[test]
fn test_guide_markdown() {
    let dir = temp_db();
    let db = db_path(&dir);

    let (stdout, _, ok) = exp(&["guide"], &db);
    assert!(ok);
    assert!(stdout.contains("Experiment Tracker"));
    assert!(stdout.contains("Workflow"));
}

#[test]
fn test_guide_json() {
    let dir = temp_db();
    let db = db_path(&dir);

    let (stdout, _, ok) = exp(&["guide", "--format", "json"], &db);
    assert!(ok);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("should be valid JSON");
    assert!(parsed["workflow_steps"].is_array());
    assert!(parsed["concepts"].is_object());
}

#[test]
fn test_plan() {
    let dir = temp_db();
    let db = db_path(&dir);

    exp(&["create", "plan-test"], &db);
    exp(&["var", "set", "plan-test", "--independent", "strategy=a,b,c"], &db);

    // Complete one run
    let (run_id, _, _) = exp(&["run", "start", "plan-test", "--strategy=a"], &db);
    exp_stdin(&["run", "record", run_id.trim(), "--output", "-"], &db, r#"{"x": 1}"#);

    let (stdout, _, ok) = exp(&["plan", "plan-test"], &db);
    assert!(ok);
    assert!(stdout.contains("#!/bin/bash"));
    assert!(stdout.contains("2 runs remaining"));
    assert!(!stdout.contains("--strategy=\"a\""), "completed run should not appear in plan");
    assert!(stdout.contains("--strategy=\"b\""));
    assert!(stdout.contains("--strategy=\"c\""));
}

#[test]
fn test_record_merges_output() {
    let dir = temp_db();
    let db = db_path(&dir);

    exp(&["create", "merge-test"], &db);

    let (run_id, _, _) = exp(&["run", "start", "merge-test"], &db);
    let run_id = run_id.trim();

    // Record first set of metrics
    exp_stdin(&["run", "record", run_id, "--output", "-"], &db, r#"{"accuracy": 0.82}"#);

    // Record more metrics — should merge
    exp_stdin(&["run", "record", run_id, "--output", "-"], &db, r#"{"tokens": 1240}"#);

    let (stdout, _, _) = exp(&["run", "show", run_id], &db);
    assert!(stdout.contains("accuracy"));
    assert!(stdout.contains("tokens"));
}

#[test]
fn test_export_json() {
    let dir = temp_db();
    let db = db_path(&dir);

    exp(&["create", "export-test"], &db);
    let (run_id, _, _) = exp(&["run", "start", "export-test"], &db);
    exp_stdin(&["run", "record", run_id.trim(), "--output", "-"], &db, r#"{"val": 42}"#);

    let (stdout, _, ok) = exp(&["export", "export-test", "--format", "json"], &db);
    assert!(ok);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert!(parsed.is_array());
}

#[test]
fn test_record_inline_json() {
    let dir = temp_db();
    let db = db_path(&dir);

    exp(&["create", "inline-test"], &db);
    let (run_id, _, _) = exp(&["run", "start", "inline-test"], &db);
    let run_id = run_id.trim();

    let (_, _, ok) = exp(&["run", "record", run_id, "--output", r#"{"score": 0.95}"#], &db);
    assert!(ok);

    let (stdout, _, _) = exp(&["run", "show", run_id], &db);
    assert!(stdout.contains("0.95"));
}
