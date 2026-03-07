use anyhow::Result;

use crate::display;

struct Template {
    name: &'static str,
    description: &'static str,
    controls: &'static [(&'static str, &'static str)],
    independents: &'static [(&'static str, &'static str)],
    expected_outputs: &'static [(&'static str, &'static str)],
    example: &'static str,
}

const TEMPLATES: &[Template] = &[
    Template {
        name: "prompt-ab",
        description: "Compare prompt variants (A/B or multi-way)",
        controls: &[("model", "claude-sonnet-4-20250514"), ("eval_set", "my-eval")],
        independents: &[("prompt_template", "baseline,variant-a,variant-b"), ("include_examples", "true,false")],
        expected_outputs: &[("score", "float"), ("tokens", "int"), ("latency_s", "float")],
        example: r#"exp create "prompt-test" --template prompt-ab
exp var set prompt-test --control model=claude-sonnet-4-20250514
exp var set prompt-test --independent prompt_template="concise,detailed,cot"
RUN=$(exp run start prompt-test --prompt_template="concise")
python eval.py --prompt concise | exp run record "$RUN" --output -
exp compare prompt-test --sort-by score"#,
    },
    Template {
        name: "model-compare",
        description: "Same task across different models",
        controls: &[("task", "my-task"), ("prompt", "standard-v1")],
        independents: &[("model", "gpt-4o,claude-sonnet-4-20250514,gemini-pro")],
        expected_outputs: &[("accuracy", "float"), ("tokens", "int"), ("cost_usd", "float"), ("latency_s", "float")],
        example: r#"exp create "model-bench" --template model-compare
exp var set model-bench --control task="summarize contracts"
exp var set model-bench --independent model="gpt-4o,claude-sonnet-4-20250514,gemini-pro"
RUN=$(exp run start model-bench --model="gpt-4o")
python eval.py --model gpt-4o | exp run record "$RUN" --output -
exp compare model-bench --sort-by accuracy"#,
    },
    Template {
        name: "strategy-sweep",
        description: "Compare agent strategies/approaches",
        controls: &[("model", "claude-sonnet-4-20250514"), ("eval_criteria", "accuracy,completeness")],
        independents: &[("strategy", "direct,cot,cot+fanout,react")],
        expected_outputs: &[("score", "float"), ("tokens", "int"), ("latency_s", "float")],
        example: r#"exp create "strategy-test" --template strategy-sweep
exp var set strategy-test --control model=claude-sonnet-4-20250514
exp var set strategy-test --independent strategy="direct,cot,react,cot+fanout"
RUN=$(exp run start strategy-test --strategy="direct")
python agent.py --strategy direct | exp run record "$RUN" --output -
exp compare strategy-test --sort-by score"#,
    },
    Template {
        name: "param-sweep",
        description: "Sweep numeric parameters",
        controls: &[("model", "claude-sonnet-4-20250514")],
        independents: &[("temperature", "0.0,0.3,0.5,0.7,1.0"), ("top_p", "0.9,0.95,1.0")],
        expected_outputs: &[("accuracy", "float"), ("diversity", "float"), ("tokens", "int")],
        example: r#"exp create "temp-sweep" --template param-sweep
exp var set temp-sweep --control model=claude-sonnet-4-20250514
exp var set temp-sweep --independent temperature="0.0,0.3,0.7,1.0"
for temp in 0.0 0.3 0.7 1.0; do
  RUN=$(exp run start temp-sweep --temperature="$temp")
  python eval.py --temp "$temp" | exp run record "$RUN" --output -
done
exp compare temp-sweep --sort-by accuracy"#,
    },
    Template {
        name: "custom",
        description: "Blank slate — no pre-set variables",
        controls: &[],
        independents: &[],
        expected_outputs: &[],
        example: r#"exp create "my-experiment" --template custom
exp var set my-experiment --control <name>=<value>
exp var set my-experiment --independent <name>="<val1>,<val2>"
RUN=$(exp run start my-experiment --<name>="<val>")
<your command> | exp run record "$RUN" --output -
exp compare my-experiment"#,
    },
];

pub fn list() -> Result<()> {
    let rows: Vec<Vec<String>> = TEMPLATES
        .iter()
        .map(|t| vec![t.name.to_string(), t.description.to_string()])
        .collect();

    let table = display::build_table(&["template", "description"], &rows);
    println!("{table}");
    Ok(())
}

pub fn show(name: &str) -> Result<()> {
    let tmpl = TEMPLATES
        .iter()
        .find(|t| t.name == name)
        .ok_or_else(|| anyhow::anyhow!("unknown template: {name}"))?;

    println!("# {} — {}\n", tmpl.name, tmpl.description);

    if !tmpl.controls.is_empty() {
        println!("Suggested controls:");
        for (k, v) in tmpl.controls {
            println!("  {k} = {v}");
        }
        println!();
    }

    if !tmpl.independents.is_empty() {
        println!("Suggested independent variables:");
        for (k, v) in tmpl.independents {
            println!("  {k} = [{v}]");
        }
        println!();
    }

    if !tmpl.expected_outputs.is_empty() {
        println!("Expected output keys:");
        for (k, t) in tmpl.expected_outputs {
            println!("  {k} ({t})");
        }
        println!();
    }

    println!("Example:\n");
    println!("{}", tmpl.example);

    Ok(())
}

/// Returns (role, name, values) tuples for a template's variables.
pub fn template_variables(template: &str) -> Vec<(&'static str, &'static str, &'static str)> {
    let Some(tmpl) = TEMPLATES.iter().find(|t| t.name == template) else {
        return vec![];
    };

    let mut result = Vec::new();
    for (k, v) in tmpl.controls {
        result.push(("control", *k, *v));
    }
    for (k, v) in tmpl.independents {
        result.push(("independent", *k, *v));
    }
    result
}
