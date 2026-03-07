use anyhow::Result;

const GUIDE_MARKDOWN: &str = r#"# exp — Experiment Tracker CLI

## Concepts

- **Experiment**: A named container for a set of runs testing a hypothesis.
- **Controls**: Variables held constant across all runs (e.g., model, dataset).
- **Independent variables**: Variables that change per run (e.g., strategy, temperature).
- **Run**: A single execution with specific variable values. Produces JSON output.
- **Output**: Structured JSON metrics recorded after a run completes.
- **Journal**: Structured JSON context stored alongside output (prompts, raw responses, filter decisions). Kept separate so `compare` tables stay clean.
- **Artifacts**: File snapshots stored in the database alongside a run.
- **Comments**: Timestamped notes on experiments or individual runs.

## Workflow

1. Create an experiment: `exp create <name>`
2. Define variables: `exp var set <name> --control key=val --independent key="a,b,c"`
3. Start a run: `RUN=$(exp run start <name> --key="val")`
4. Record results: `exp run record "$RUN" --output '{"accuracy": 0.9}' --journal '{"prompt": "...", "raw_response": "..."}'`
5. Add comments: `exp run comment "$RUN" "observation"`
6. Compare runs: `exp compare <name> --sort-by <metric>`

## Quick Start

```bash
exp create "my-test"
exp var set my-test --control model=claude-sonnet-4-20250514
exp var set my-test --independent strategy="direct,cot"

RUN=$(exp run start my-test --strategy="direct")
echo '{"accuracy": 0.82, "tokens": 1240}' | exp run record "$RUN" --output -

RUN=$(exp run start my-test --strategy="cot")
echo '{"accuracy": 0.91, "tokens": 2400}' | exp run record "$RUN" --output -

exp compare my-test --sort-by accuracy
```

## Templates

Use `exp templates` to see pre-built experiment shapes.
Use `exp create <name> --template <template>` to scaffold from a template.

## Agent Discovery

- `exp guide` — this walkthrough
- `exp templates` — list available templates
- `exp describe <experiment>` — introspect an existing experiment
- `exp plan <experiment>` — generate a shell script for remaining runs

## Commands Reference

| Command | Purpose |
|---|---|
| `exp create <name>` | Create a new experiment |
| `exp list` | List all experiments |
| `exp status <name>` | Show experiment status |
| `exp delete <name>` | Delete an experiment |
| `exp var set <name>` | Set control/independent variables |
| `exp var list <name>` | List variables |
| `exp var rm <name> <var>` | Remove a variable |
| `exp run start <name>` | Start a run with variable values |
| `exp run record <run-id>` | Record JSON output (and optional --journal) for a run |
| `exp run fail <run-id>` | Mark a run as failed |
| `exp run comment <run-id> <text>` | Comment on a run |
| `exp run artifact <run-id> <file>` | Attach a file to a run |
| `exp run list <name>` | List runs for an experiment |
| `exp run show <run-id>` | Show full run details |
| `exp compare <name>` | Compare runs side by side |
| `exp export <name>` | Export data as CSV or JSON |
| `exp comment <name> <text>` | Comment on the experiment |
| `exp comments <name>` | List all comments |
| `exp describe <name>` | Introspect experiment for agents |
| `exp plan <name>` | Generate run script |
| `exp guide` | This walkthrough |
| `exp templates` | List templates |
"#;

pub fn run(format: &str) -> Result<()> {
    match format {
        "json" => {
            let guide = serde_json::json!({
                "workflow_steps": [
                    {"order": 1, "command": "exp create <name>", "purpose": "Create a new experiment"},
                    {"order": 2, "command": "exp var set <name> --control key=val --independent key=\"a,b\"", "purpose": "Define control and independent variables"},
                    {"order": 3, "command": "RUN=$(exp run start <name> --key=\"val\")", "purpose": "Start a run with specific variable values. Capture the run ID."},
                    {"order": 4, "command": "exp run record \"$RUN\" --output '{...}' --journal '{...}'", "purpose": "Record metrics (output) and context (journal) for the run"},
                    {"order": 5, "command": "exp run comment \"$RUN\" \"text\"", "purpose": "Add observations to a run"},
                    {"order": 6, "command": "exp compare <name> --sort-by <metric>", "purpose": "Compare all runs side by side"}
                ],
                "concepts": {
                    "controls": "Variables held constant across all runs (e.g., model, dataset).",
                    "independents": "Variables that change per run (e.g., strategy, temperature).",
                    "outputs": "Structured JSON metrics recorded after each run.",
                    "journal": "Structured JSON context stored alongside output (prompts, raw responses, decisions). Separate from output so compare stays clean.",
                    "artifacts": "File snapshots stored in the database alongside a run."
                },
                "output_schema": {
                    "description": "Any valid JSON object. Top-level keys become columns in exp compare.",
                    "example": {"accuracy": 0.92, "tokens": 1240, "latency_s": 3.2}
                },
                "discovery_commands": [
                    "exp guide --format json",
                    "exp templates",
                    "exp describe <experiment>",
                    "exp plan <experiment>"
                ]
            });
            println!("{}", serde_json::to_string_pretty(&guide)?);
        }
        _ => {
            print!("{GUIDE_MARKDOWN}");
        }
    }

    Ok(())
}
