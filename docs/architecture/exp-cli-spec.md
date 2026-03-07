# exp — Experiment Tracker CLI Specification

## Overview

`exp` is a Rust CLI for creating experiments, tracking variables, recording
results, and comparing runs. It is designed to be used by both humans and
autonomous agents. An agent with no prior knowledge of `exp` can discover its
full capabilities at runtime through built-in guidance commands.

The tool is domain-agnostic — it works for LLM prompt testing, agent strategy
evaluation, parameter sweeps, and scientific simulations.

---

## Data Model

```
Experiment
├── id            (ULID, auto-generated)
├── name          (unique, user-provided, used as CLI handle)
├── description   (optional free text / hypothesis)
├── template      (optional, template name used to scaffold)
├── created_at
├── status        (draft | running | completed | failed)
│
├── Variables[]
│   ├── name
│   ├── role      (control | independent)
│   └── values    (comma-separated string of allowed values)
│
├── Runs[]
│   ├── run_id       (ULID)
│   ├── status       (pending | running | completed | failed)
│   ├── variables    (key=value pairs for this run)
│   ├── started_at
│   ├── finished_at
│   ├── output       (structured JSON blob — metrics, scores, etc.)
│   ├── artifacts[]  (binary blobs: logs, transcripts, model outputs)
│   └── comments[]   (timestamped text notes)
│
└── Comments[]       (experiment-level timestamped notes)
```

### Storage

All data lives in a single SQLite database file. Default location:
`.exp/experiments.db` relative to the current working directory.

Override with `EXP_DB` environment variable or `--db <path>` global flag.

Artifacts (log files, prompt files, model outputs) are stored as blobs in the
database, not as external file references. This makes the database fully
self-contained and portable.

### Schema

```sql
CREATE TABLE experiments (
    id          TEXT PRIMARY KEY,
    name        TEXT UNIQUE NOT NULL,
    description TEXT,
    template    TEXT,
    status      TEXT NOT NULL DEFAULT 'draft',
    created_at  TEXT NOT NULL
);

CREATE TABLE variables (
    id      TEXT PRIMARY KEY,
    exp_id  TEXT NOT NULL REFERENCES experiments(id),
    name    TEXT NOT NULL,
    role    TEXT NOT NULL CHECK (role IN ('control', 'independent')),
    values  TEXT,
    UNIQUE(exp_id, name)
);

CREATE TABLE runs (
    id          TEXT PRIMARY KEY,
    exp_id      TEXT NOT NULL REFERENCES experiments(id),
    status      TEXT NOT NULL DEFAULT 'pending',
    started_at  TEXT,
    finished_at TEXT,
    output      TEXT  -- JSON blob
);

CREATE TABLE run_variables (
    run_id   TEXT NOT NULL REFERENCES runs(id),
    var_name TEXT NOT NULL,
    value    TEXT NOT NULL,
    PRIMARY KEY (run_id, var_name)
);

CREATE TABLE artifacts (
    id       TEXT PRIMARY KEY,
    run_id   TEXT NOT NULL REFERENCES runs(id),
    name     TEXT NOT NULL,
    content  BLOB NOT NULL,
    added_at TEXT NOT NULL
);

CREATE TABLE comments (
    id        TEXT PRIMARY KEY,
    exp_id    TEXT REFERENCES experiments(id),
    run_id    TEXT REFERENCES runs(id),
    body      TEXT NOT NULL,
    added_at  TEXT NOT NULL
);
```

A comment has either `exp_id` or `run_id` set, never both.

---

## CLI Commands

All commands operate on the database in the current working directory unless
overridden. Experiment names are used as handles wherever `<experiment>` appears.

### Experiment Lifecycle

#### `exp create <name>`

Create a new experiment.

```
exp create "cot-eval"
exp create "cot-eval" --description "Test chain-of-thought on legal docs"
exp create "cot-eval" --template strategy-sweep
```

Flags:
- `--description <text>` — hypothesis or purpose
- `--template <name>` — scaffold from a built-in template

Prints the experiment id. Sets status to `draft`.

#### `exp status <experiment>`

Show experiment status: variable definitions, run count, completion progress.

#### `exp list`

List all experiments in the database.

```
exp list
exp list --status running
```

#### `exp delete <experiment>`

Delete an experiment and all associated runs, variables, artifacts, and
comments. Prompts for confirmation unless `--force` is passed.

---

### Variables

Variables are untyped key=string pairs. The system does not distinguish between
quantitative and qualitative values. Numeric sorting is applied opportunistically
at display time when all values for a column parse as numbers.

#### `exp var set <experiment>`

Define or update a variable on an experiment.

```
exp var set cot-eval --control model=claude-sonnet-4-20250514
exp var set cot-eval --control dataset=eval-v3
exp var set cot-eval --independent strategy="direct,cot,cot+fanout,react"
exp var set cot-eval --independent fanout_width="3,5,n/a"
```

Flags:
- `--control <name>=<value>` — constant across all runs
- `--independent <name>=<value>,<value>,...` — varies per run

Repeatable. Multiple `--control` and `--independent` flags can appear in one
invocation.

#### `exp var list <experiment>`

List all variables defined on an experiment, grouped by role.

#### `exp var rm <experiment> <name>`

Remove a variable definition.

---

### Runs

#### `exp run start <experiment>`

Start a new run. Variable values for this run are passed as flags matching
the independent variable names.

```
RUN=$(exp run start cot-eval --strategy="cot" --fanout_width="n/a")
```

Prints the run id to stdout (and nothing else) so it can be captured in a
shell variable. Sets run status to `running` and records `started_at`.

Unknown variable names passed as flags are accepted and stored — this allows
ad-hoc variables without requiring prior definition.

#### `exp run record <run-id>`

Record structured output for a run.

```
# from a file
exp run record "$RUN" --output results.json

# from stdin
python eval.py | exp run record "$RUN" --output -

# inline
exp run record "$RUN" --output '{"accuracy": 0.92, "tokens": 1240}'
```

The output must be valid JSON. It is stored as-is in the `runs.output` column.
Calling `record` multiple times on the same run merges the JSON objects
(top-level keys from later calls overwrite earlier ones).

Sets run status to `completed` and records `finished_at`.

#### `exp run fail <run-id>`

Mark a run as failed. Optionally attach an error message.

```
exp run fail "$RUN" --reason "OOM at batch 47"
```

#### `exp run comment <run-id> <text>`

Add a timestamped comment to a run.

```
exp run comment "$RUN" "cot +7% accuracy but 2x tokens"
```

#### `exp run artifact <run-id> <file-path>`

Snapshot a file into the database as an artifact attached to this run.

```
exp run artifact "$RUN" prompts/cot-v2.txt
exp run artifact "$RUN" logs/run-output.log
```

The file content is read and stored as a blob. The original filename is
preserved as metadata.

#### `exp run list <experiment>`

List all runs for an experiment with their status and variable values.

#### `exp run show <run-id>`

Show full details of a run: variables, output, artifacts, comments.

---

### Comparison and Reporting

#### `exp compare <experiment>`

Display a table of all completed runs with their variable values and output
metrics side by side.

```
exp compare cot-eval
exp compare cot-eval --sort-by accuracy
exp compare cot-eval --sort-by accuracy --desc
exp compare cot-eval --group-by strategy
exp compare cot-eval --where strategy~fanout
exp compare cot-eval --where "tokens<2000"
exp compare cot-eval --cols strategy,accuracy,tokens
```

Flags:
- `--sort-by <key>` — sort by an output metric key
- `--desc` — descending sort (default is ascending)
- `--group-by <var>` — group rows by a variable value
- `--where <expr>` — filter runs. Supports: `=`, `!=`, `<`, `>`, `~` (contains)
- `--cols <list>` — select which variable/output columns to display
- `--format table|csv|json` — output format (default: `table`)

Table output uses aligned columns with box-drawing characters. Numeric columns
are right-aligned. Columns are auto-detected from the union of all run
variable names and output JSON keys.

#### `exp export <experiment>`

Export full experiment data.

```
exp export cot-eval --format json > results.json
exp export cot-eval --format csv > results.csv
```

---

### Experiment-Level Comments

#### `exp comment <experiment> <text>`

Add a timestamped comment to the experiment itself (not a specific run).

```
exp comment cot-eval "switching to eval-v4 dataset for remaining runs"
```

#### `exp comments <experiment>`

List all comments (both experiment-level and per-run).

---

### Agent Discovery

These commands allow an agent to learn how to use `exp` at runtime without
prior documentation.

#### `exp guide`

Print a full walkthrough of concepts, workflow, and examples.

```
exp guide                  # markdown (default)
exp guide --format json    # structured for programmatic consumption
```

The JSON format includes:

```json
{
  "workflow_steps": [
    {"order": 1, "command": "exp create <name>", "purpose": "..."},
    ...
  ],
  "concepts": {
    "controls": "Variables held constant across all runs.",
    "independents": "Variables that change per run.",
    "outputs": "Structured JSON metrics recorded after each run.",
    "artifacts": "File snapshots stored in the database."
  },
  "output_schema": {
    "description": "Any valid JSON object. Keys become columns in compare.",
    "example": {"accuracy": 0.92, "tokens": 1240, "latency_s": 3.2}
  },
  "examples": [...]
}
```

#### `exp templates`

List available experiment templates.

```
exp templates              # list all
exp templates show <name>  # show template details with example commands
```

Built-in templates:

| Name | Description |
|---|---|
| `prompt-ab` | Compare prompt variants (A/B or multi-way) |
| `model-compare` | Same task across different models |
| `strategy-sweep` | Compare agent strategies/approaches |
| `param-sweep` | Sweep numeric parameters |
| `custom` | Blank — no pre-set variables |

Templates are embedded in the binary. Each template defines:
- Suggested control variable names
- Suggested independent variable names with example values
- Expected output JSON keys with type hints
- A copy-pasteable example workflow

#### `exp describe <experiment>`

Introspect an existing experiment. Outputs its current state, what's been run,
and what remains. Designed for an agent to read mid-experiment.

```
$ exp describe cot-eval

Experiment: cot-eval (01JARQ...)
Status: running (2/8 runs completed)

Controls:
  model   = claude-sonnet-4-20250514
  dataset = eval-v3

Independent variables:
  strategy     = [direct, cot, cot+fanout, react]
  fanout_width = [3, 5, n/a]

Expected output keys (from completed runs):
  accuracy (float), tokens (int), latency_s (float)

Completed runs:
  run-a3x: strategy=direct, fanout_width=n/a
  run-b7f: strategy=cot, fanout_width=n/a

Remaining combinations:
  --strategy="cot+fanout" --fanout_width="3"
  --strategy="cot+fanout" --fanout_width="5"
  --strategy="react" --fanout_width="3"
  --strategy="react" --fanout_width="5"
  --strategy="direct" --fanout_width="3"
  --strategy="direct" --fanout_width="5"

To start the next run:
  RUN=$(exp run start cot-eval --strategy="cot+fanout" --fanout_width="3")
  <your command> | exp run record "$RUN" --output -
```

Also supports `--format json`.

#### `exp plan <experiment>`

Generate a shell script to execute all remaining runs.

```
$ exp plan cot-eval --shell bash

#!/bin/bash
# Run plan for: cot-eval
# 6 runs remaining

RUN=$(exp run start cot-eval --strategy="cot+fanout" --fanout_width="3")
# TODO: replace with your command
YOUR_COMMAND | exp run record "$RUN" --output -

RUN=$(exp run start cot-eval --strategy="cot+fanout" --fanout_width="5")
YOUR_COMMAND | exp run record "$RUN" --output -

# ...
```

The agent (or human) fills in `YOUR_COMMAND` and executes the script.

---

## Script Integration

`exp` communicates via stdout, stdin, and exit codes. Any language or tool can
integrate with it.

### Pattern: Wrapper Script

```bash
#!/bin/bash
set -euo pipefail

EXP="retrieval-qa"
exp create "$EXP" --template strategy-sweep 2>/dev/null || true
exp var set "$EXP" --control model=claude-sonnet-4-20250514
exp var set "$EXP" --independent strategy="1-shot,1-shot+fanout,cot"

for strategy in 1-shot 1-shot+fanout cot; do
    RUN=$(exp run start "$EXP" --strategy="$strategy")
    if python agent.py --strategy "$strategy" > /tmp/result.json 2>/tmp/err.log; then
        exp run record "$RUN" --output /tmp/result.json
        exp run artifact "$RUN" /tmp/err.log
    else
        exp run fail "$RUN" --reason "$(tail -1 /tmp/err.log)"
    fi
done

exp compare "$EXP" --sort-by accuracy
```

### Pattern: Python Integration

```python
import subprocess, json

def exp(*args):
    r = subprocess.run(["exp"] + list(args), capture_output=True, text=True)
    return r.stdout.strip()

run_id = exp("run", "start", "my-exp", "--temp=0.7")
results = run_evaluation(temp=0.7)
exp("run", "record", run_id, "--output", json.dumps(results))
```

### Pattern: Agent Self-Directed

An agent given the instruction "use exp to run this experiment" would:

1. Run `exp guide --format json` to learn the tool
2. Run `exp templates` to find a matching template
3. Create the experiment and set variables
4. Run `exp plan <name>` to generate the run script
5. Execute each run, recording results
6. Run `exp compare <name>` to analyze
7. Add comments with observations

---

## Technical Decisions

| Decision | Rationale |
|---|---|
| Rust, single binary | No runtime dependencies. Fast. Matches team preference. |
| SQLite (rusqlite) | Zero config, single file, portable, queryable. |
| Artifacts as blobs | Self-contained DB. No broken file references. |
| ULID identifiers | Time-sortable, no coordination, copy-pasteable. |
| Untyped variables | Supports numeric, categorical, and file-reference variables uniformly. Numeric parsing applied only at display time. |
| JSON output blobs | Flexible schema per experiment. No migrations when metrics change. |
| Stdin/stdout interface | Language-agnostic. Composable with pipes and shell scripts. |
| Embedded templates | Travel with the binary. No config files to locate. |
| run start prints only ID | Capturable with `$()` in shell scripts. |

## Crate Dependencies

| Crate | Purpose |
|---|---|
| `clap` | CLI argument parsing with derive macros |
| `rusqlite` (bundled) | SQLite storage |
| `serde`, `serde_json` | JSON serialization |
| `ulid` | ID generation |
| `comfy-table` | Terminal table rendering |
| `chrono` | Timestamps |

## Project Layout

```
exp/
├── Cargo.toml
├── src/
│   ├── main.rs              # CLI entry point (clap)
│   ├── db.rs                # Schema init, connection management
│   ├── models.rs            # Experiment, Run, Variable, Comment structs
│   ├── commands/
│   │   ├── mod.rs
│   │   ├── create.rs        # exp create
│   │   ├── list.rs          # exp list
│   │   ├── status.rs        # exp status
│   │   ├── delete.rs        # exp delete
│   │   ├── var.rs           # exp var set/list/rm
│   │   ├── run.rs           # exp run start/record/fail/comment/artifact/list/show
│   │   ├── compare.rs       # exp compare
│   │   ├── export.rs        # exp export
│   │   ├── comment.rs       # exp comment / exp comments
│   │   ├── guide.rs         # exp guide
│   │   ├── templates.rs     # exp templates
│   │   ├── describe.rs      # exp describe
│   │   └── plan.rs          # exp plan
│   └── display.rs           # Table formatting, column detection
├── docs/
│   └── architecture/
│       └── exp-cli-spec.md  # this file
└── migrations/
    └── 001_init.sql         # schema (also embedded in db.rs)
```

## Exit Codes

| Code | Meaning |
|---|---|
| 0 | Success |
| 1 | General error (invalid args, db error) |
| 2 | Experiment not found |
| 3 | Run not found |
| 4 | Invalid JSON in output |
