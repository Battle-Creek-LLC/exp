# exp

A CLI experiment tracker for agent runs, prompt testing, and simulations.

`exp` helps you create experiments, define variables, execute runs, record structured results, and compare outcomes — all from the command line. It's designed to be used by both humans and autonomous agents.

## Features

- **Domain-agnostic** — works for LLM prompt testing, agent strategy evaluation, parameter sweeps, and scientific simulations
- **Self-contained storage** — single SQLite database file with artifacts stored as blobs
- **Agent-friendly** — built-in `guide` and `describe` commands let agents discover capabilities at runtime
- **Composable** — stdin/stdout interface works with shell scripts, Python, or any language
- **Zero config** — single binary, no runtime dependencies

## Installation

### From source

```bash
cargo install --path .
```

### Build from source

```bash
git clone https://github.com/jstockdi/exp.git
cd exp
cargo build --release
# Binary is at target/release/exp
```

## Quick Start

```bash
# Create an experiment
exp create "cot-eval" --description "Test chain-of-thought on legal docs"

# Define variables
exp var set cot-eval --control model=claude-sonnet-4-20250514
exp var set cot-eval --independent strategy="direct,cot,cot+fanout"

# Run and record results
RUN=$(exp run start cot-eval --strategy="direct")
# ... run your evaluation ...
exp run record "$RUN" --output '{"accuracy": 0.85, "tokens": 940}'

RUN=$(exp run start cot-eval --strategy="cot")
# ... run your evaluation ...
exp run record "$RUN" --output '{"accuracy": 0.92, "tokens": 1240}'

# Compare results
exp compare cot-eval --sort-by accuracy
```

## Usage

### Experiment Lifecycle

```bash
exp create <name>                    # Create a new experiment
exp list [--status <status>]         # List experiments
exp status <experiment>              # Show experiment status
exp delete <experiment> [--force]    # Delete an experiment
```

### Variables

```bash
exp var set <experiment> --control <name>=<value>           # Set a constant
exp var set <experiment> --independent <name>="val1,val2"   # Set a variable that changes per run
exp var list <experiment>                                    # List variables
exp var rm <experiment> <name>                               # Remove a variable
```

### Runs

```bash
exp run start <experiment> --key=value    # Start a run (prints run ID)
exp run record <run-id> --output <json>   # Record results (file, stdin with -, or inline JSON)
exp run fail <run-id> [--reason <text>]   # Mark a run as failed
exp run comment <run-id> <text>           # Add a comment to a run
exp run artifact <run-id> <file>          # Attach a file artifact
exp run list <experiment>                 # List runs
exp run show <run-id>                     # Show run details
```

### Analysis

```bash
exp compare <experiment>                  # Compare all runs side by side
exp compare <experiment> --sort-by accuracy --desc
exp compare <experiment> --where "tokens<2000"
exp compare <experiment> --format csv
exp export <experiment> --format json     # Export full data
```

### Agent Discovery

```bash
exp guide                     # Full usage walkthrough
exp guide --format json       # Structured for programmatic consumption
exp templates                 # List built-in templates
exp describe <experiment>     # Introspect current state and remaining work
exp plan <experiment>         # Generate shell script for remaining runs
```

## Templates

| Name | Description |
|---|---|
| `prompt-ab` | Compare prompt variants (A/B or multi-way) |
| `model-compare` | Same task across different models |
| `strategy-sweep` | Compare agent strategies/approaches |
| `param-sweep` | Sweep numeric parameters |
| `custom` | Blank — no pre-set variables |

```bash
exp create my-test --template prompt-ab
```

## Storage

All data lives in a single SQLite file at `.exp/experiments.db` (relative to the working directory). Override with `EXP_DB` env var or `--db <path>`.

## License

MIT
