# Using exp with an AI Agent

This guide shows how to instruct an AI agent to use `exp` for structured experimentation. The agent needs no prior knowledge of `exp` — it can discover the tool's capabilities at runtime.

## Setup

Add the following to your agent's system prompt or project instructions (e.g., `CLAUDE.md`):

```markdown
## Experiment Tracking

Use the `exp` CLI to track experiments. Run `exp guide --format json` to learn
the tool, then use it to create experiments, record results, and compare runs.

Always capture run IDs: `RUN=$(exp run start ...)`
Always record structured JSON output: `exp run record "$RUN" --output '{"metric": value}'`
```

That's it. The agent can discover everything else through `exp guide` and `exp templates`.

## What the Agent Does

When given an experimental task (e.g., "test which prompt strategy works best for summarization"), an agent using `exp` will typically:

### 1. Discover the tool

```bash
exp guide --format json
```

The JSON output gives the agent a structured understanding of the workflow, concepts, and available commands.

### 2. Find a matching template

```bash
exp templates
exp templates show strategy-sweep
```

Templates suggest variable names, expected output keys, and provide example commands.

### 3. Create the experiment

```bash
exp create "summarization-strategies" --template strategy-sweep
```

### 4. Define variables

```bash
exp var set summarization-strategies \
  --control model=claude-sonnet-4-20250514 \
  --control dataset=contracts-50 \
  --independent strategy="direct,cot,cot+examples"
```

### 5. Check what needs to run

```bash
exp describe summarization-strategies
```

This shows the agent what combinations have been completed and what remains — useful for resuming after interruptions.

### 6. Execute runs

```bash
RUN=$(exp run start summarization-strategies --strategy="direct")

# Agent runs its evaluation here and captures metrics...

exp run record "$RUN" --output '{"accuracy": 0.72, "tokens": 840, "latency_s": 1.2}'
exp run comment "$RUN" "Baseline performance. Struggles with nested clauses."
```

If a run fails:

```bash
exp run fail "$RUN" --reason "API rate limit exceeded"
```

### 7. Generate a run plan

For larger experiments, the agent can generate a script for all remaining runs:

```bash
exp plan summarization-strategies --shell bash
```

This outputs a ready-to-execute script with placeholders for the evaluation command.

### 8. Compare and analyze

```bash
exp compare summarization-strategies --sort-by accuracy --desc
```

The agent reads the comparison table and draws conclusions.

### 9. Record observations

```bash
exp comment summarization-strategies "cot+examples achieves 94% accuracy — recommend for production"
```

## Example: Full Agent Session

Here's what a typical agent session looks like end-to-end:

```
User: Test whether chain-of-thought improves accuracy on our contract
      summarization task. Try direct prompting, CoT, and CoT with examples.

Agent: I'll set up a structured experiment to compare these strategies.

> exp guide --format json
> exp create "cot-contract-eval" --description "Compare prompting strategies for contract summarization"
> exp var set cot-contract-eval --control model=claude-sonnet-4-20250514 --control dataset=contracts-50
> exp var set cot-contract-eval --independent strategy="direct,cot,cot+examples"

I've created the experiment. Let me run each strategy.

> RUN=$(exp run start cot-contract-eval --strategy="direct")
> # ... agent runs evaluation ...
> exp run record "$RUN" --output '{"accuracy": 0.72, "precision": 0.68, "recall": 0.78, "tokens": 840}'
> exp run comment "$RUN" "Misses indemnification clauses in 3/10 cases"

> RUN=$(exp run start cot-contract-eval --strategy="cot")
> # ... agent runs evaluation ...
> exp run record "$RUN" --output '{"accuracy": 0.89, "precision": 0.87, "recall": 0.91, "tokens": 2100}'
> exp run comment "$RUN" "Reasoning helps with complex clause structures"

> RUN=$(exp run start cot-contract-eval --strategy="cot+examples")
> # ... agent runs evaluation ...
> exp run record "$RUN" --output '{"accuracy": 0.94, "precision": 0.93, "recall": 0.95, "tokens": 3400}'
> exp run comment "$RUN" "Examples resolve ambiguous edge cases"

> exp compare cot-contract-eval --sort-by accuracy --desc

Results:

| strategy     | accuracy | precision | recall | tokens |
|-------------|----------|-----------|--------|--------|
| cot+examples | 0.94     | 0.93      | 0.95   | 3400   |
| cot          | 0.89     | 0.87      | 0.91   | 2100   |
| direct       | 0.72     | 0.68      | 0.78   | 840    |

> exp comment cot-contract-eval "CoT+examples is the clear winner at 94% accuracy. The 4x token increase over direct prompting is justified by the 22-point accuracy gain."

Agent: Chain-of-thought with examples achieves the best results (94% accuracy).
       The token cost is higher (3400 vs 840 for direct), but the accuracy
       improvement is substantial. I recommend cot+examples for production use.
       Full results are stored in the experiment — run `exp describe cot-contract-eval`
       to review.
```

## Tips for Agent Integration

- **Resumability**: If an agent is interrupted, it can run `exp describe <name>` to see what's been completed and pick up where it left off.
- **Failure handling**: Agents should use `exp run fail` with `--reason` when runs error out, rather than silently skipping them.
- **Artifacts**: Agents can attach relevant files (prompts, logs, raw outputs) with `exp run artifact` for later review.
- **Journal**: Use `--journal` in `exp run record` to store context (prompts, raw responses) separately from metrics. This keeps the `exp compare` table clean while preserving full context.
- **Export**: Use `exp export <name> --format json` to get machine-readable results for further analysis.
