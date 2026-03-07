#!/bin/bash
set -euo pipefail

# =============================================================================
# Prompt Evaluation Example
# =============================================================================
#
# This script demonstrates a complete exp workflow: creating an experiment,
# defining variables, running multiple prompt strategies, recording results,
# and comparing outcomes.
#
# No external dependencies — results are simulated with realistic-looking data
# so you can see the full lifecycle without needing an LLM API.
#
# Usage:
#   chmod +x examples/prompt-eval.sh
#   ./examples/prompt-eval.sh
#
# The experiment database is created in a temp directory and cleaned up on exit.
# =============================================================================

# Use a temp directory so we don't pollute the user's workspace
WORK_DIR=$(mktemp -d)
trap 'rm -rf "$WORK_DIR"' EXIT
export EXP_DB="$WORK_DIR/experiments.db"

echo "=== Prompt Strategy Evaluation ==="
echo ""

# --- Step 1: Create the experiment ---
echo "1. Creating experiment..."
exp create "prompt-eval" --description "Compare prompt strategies for contract summarization"
echo ""

# --- Step 2: Define variables ---
echo "2. Setting up variables..."
exp var set prompt-eval \
  --control model=claude-sonnet-4-20250514 \
  --control dataset=contracts-50 \
  --independent strategy="direct,cot,cot+examples,react"

exp var list prompt-eval
echo ""

# --- Step 3: Run each strategy ---
echo "3. Running evaluations..."
echo ""

# Strategy: direct — fast but lower accuracy
RUN=$(exp run start prompt-eval --strategy="direct")
echo "   Started run $RUN (strategy=direct)"
exp run record "$RUN" --output '{"accuracy": 0.72, "tokens": 840, "latency_s": 1.2, "cost_usd": 0.003}'
exp run comment "$RUN" "Baseline. Misses nuanced clauses."
echo "   Recorded results."

# Strategy: cot — better accuracy, more tokens
RUN=$(exp run start prompt-eval --strategy="cot")
echo "   Started run $RUN (strategy=cot)"
exp run record "$RUN" --output '{"accuracy": 0.89, "tokens": 2100, "latency_s": 3.1, "cost_usd": 0.008}'
exp run comment "$RUN" "Good improvement. Catches liability clauses now."
echo "   Recorded results."

# Strategy: cot+examples — best accuracy
RUN=$(exp run start prompt-eval --strategy="cot+examples")
echo "   Started run $RUN (strategy=cot+examples)"
exp run record "$RUN" --output '{"accuracy": 0.94, "tokens": 3400, "latency_s": 4.8, "cost_usd": 0.013}'
exp run comment "$RUN" "Best accuracy. Examples help with edge cases."
echo "   Recorded results."

# Strategy: react — good but expensive
RUN=$(exp run start prompt-eval --strategy="react")
echo "   Started run $RUN (strategy=react)"
exp run record "$RUN" --output '{"accuracy": 0.91, "tokens": 4200, "latency_s": 6.3, "cost_usd": 0.016}'
exp run comment "$RUN" "Iterative approach. Good accuracy but high cost."
echo "   Recorded results."

echo ""

# --- Step 4: Add an experiment-level observation ---
exp comment prompt-eval "cot+examples is the sweet spot — 94% accuracy at reasonable cost"

# --- Step 5: Compare results ---
echo "4. Comparing results (sorted by accuracy, descending)..."
echo ""
exp compare prompt-eval --sort-by accuracy --desc

echo ""
echo "5. Comparing results (sorted by cost)..."
echo ""
exp compare prompt-eval --sort-by cost_usd

echo ""
echo "6. Experiment status:"
echo ""
exp describe prompt-eval

echo ""
echo "7. Export as JSON:"
echo ""
exp export prompt-eval --format json

echo ""
echo "=== Done! ==="
echo "Database was at: $EXP_DB (cleaned up on exit)"
