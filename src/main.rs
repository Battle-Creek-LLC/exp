mod commands;
mod db;
mod display;
mod models;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "exp", about = "Experiment tracker for agent runs, prompt testing, and simulations")]
struct Cli {
    /// Path to database file (default: .exp/experiments.db)
    #[arg(long, global = true)]
    db: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new experiment
    Create {
        /// Experiment name
        name: String,
        /// Description or hypothesis
        #[arg(long)]
        description: Option<String>,
        /// Scaffold from a built-in template
        #[arg(long)]
        template: Option<String>,
    },

    /// List all experiments
    List {
        /// Filter by status (draft, running, completed, failed)
        #[arg(long)]
        status: Option<String>,
    },

    /// Show experiment status
    Status {
        /// Experiment name or id
        experiment: String,
    },

    /// Delete an experiment and all its data
    Delete {
        /// Experiment name or id
        experiment: String,
        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
    },

    /// Manage variables
    Var {
        #[command(subcommand)]
        command: VarCommands,
    },

    /// Manage runs
    Run {
        #[command(subcommand)]
        command: RunCommands,
    },

    /// Compare runs side by side
    Compare {
        /// Experiment name or id
        experiment: String,
        /// Sort by output metric
        #[arg(long)]
        sort_by: Option<String>,
        /// Sort descending
        #[arg(long)]
        desc: bool,
        /// Group rows by variable
        #[arg(long)]
        group_by: Option<String>,
        /// Filter runs (e.g. strategy=cot, tokens<2000, strategy~fanout)
        #[arg(long = "where")]
        where_clauses: Vec<String>,
        /// Select columns (comma-separated)
        #[arg(long)]
        cols: Option<String>,
        /// Output format: table, csv, json
        #[arg(long, default_value = "table")]
        format: String,
    },

    /// Export experiment data
    Export {
        /// Experiment name or id
        experiment: String,
        /// Output format: csv, json
        #[arg(long, default_value = "json")]
        format: String,
    },

    /// Add a comment to an experiment
    Comment {
        /// Experiment name or id
        experiment: String,
        /// Comment text
        text: String,
    },

    /// List all comments for an experiment
    Comments {
        /// Experiment name or id
        experiment: String,
    },

    /// Show usage guide (for humans and agents)
    Guide {
        /// Output format: markdown, json
        #[arg(long, default_value = "markdown")]
        format: String,
    },

    /// List or show built-in experiment templates
    Templates {
        #[command(subcommand)]
        command: Option<TemplateCommands>,
    },

    /// Introspect an experiment (status, remaining runs)
    Describe {
        /// Experiment name or id
        experiment: String,
        /// Output format: text, json
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Generate a shell script for remaining runs
    Plan {
        /// Experiment name or id
        experiment: String,
        /// Shell type: bash, zsh
        #[arg(long, default_value = "bash")]
        shell: String,
    },
}

#[derive(Subcommand)]
enum VarCommands {
    /// Set control or independent variables
    Set {
        /// Experiment name or id
        experiment: String,
        /// Control variable (constant): name=value
        #[arg(long)]
        control: Vec<String>,
        /// Independent variable (varies): name="val1,val2,val3"
        #[arg(long)]
        independent: Vec<String>,
    },
    /// List variables for an experiment
    List {
        /// Experiment name or id
        experiment: String,
    },
    /// Remove a variable
    Rm {
        /// Experiment name or id
        experiment: String,
        /// Variable name
        name: String,
    },
}

#[derive(Subcommand)]
enum RunCommands {
    /// Start a new run with variable values
    Start {
        /// Experiment name or id
        experiment: String,
        /// Variable values as --key=value (any flags not recognized by clap are captured)
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        vars: Vec<String>,
    },
    /// Record JSON output for a run
    Record {
        /// Run id
        run_id: String,
        /// Output source: file path, "-" for stdin, or inline JSON
        #[arg(long)]
        output: String,
    },
    /// Mark a run as failed
    Fail {
        /// Run id
        run_id: String,
        /// Error reason
        #[arg(long)]
        reason: Option<String>,
    },
    /// Add a comment to a run
    Comment {
        /// Run id
        run_id: String,
        /// Comment text
        text: String,
    },
    /// Attach a file artifact to a run
    Artifact {
        /// Run id
        run_id: String,
        /// Path to file
        file: String,
    },
    /// List runs for an experiment
    List {
        /// Experiment name or id
        experiment: String,
    },
    /// Show full details of a run
    Show {
        /// Run id
        run_id: String,
    },
}

#[derive(Subcommand)]
enum TemplateCommands {
    /// Show details of a template
    Show {
        /// Template name
        name: String,
    },
}

fn parse_key_value(s: &str) -> Option<(String, String)> {
    let s = s.trim();
    let s = s.strip_prefix("--").unwrap_or(s);
    let s = s.strip_prefix('-').unwrap_or(s);
    s.split_once('=').map(|(k, v)| (k.to_string(), v.to_string()))
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let db_path = cli.db.unwrap_or_else(|| {
        std::env::var("EXP_DB")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(".exp/experiments.db"))
    });

    // Commands that don't need a database
    match &cli.command {
        Commands::Guide { format } => return commands::guide::run(format),
        Commands::Templates { command: None } => return commands::templates::list(),
        Commands::Templates { command: Some(TemplateCommands::Show { name }) } => {
            return commands::templates::show(name);
        }
        _ => {}
    }

    let conn = db::open(&db_path)?;

    match cli.command {
        Commands::Create { name, description, template } => {
            commands::create::run(&conn, &name, description.as_deref(), template.as_deref())
        }
        Commands::List { status } => {
            commands::list::run(&conn, status.as_deref())
        }
        Commands::Status { experiment } => {
            commands::status::run(&conn, &experiment)
        }
        Commands::Delete { experiment, force } => {
            commands::delete::run(&conn, &experiment, force)
        }
        Commands::Var { command } => match command {
            VarCommands::Set { experiment, control, independent } => {
                let controls: Vec<(String, String)> = control
                    .iter()
                    .filter_map(|s| parse_key_value(s))
                    .collect();
                let independents: Vec<(String, String)> = independent
                    .iter()
                    .filter_map(|s| parse_key_value(s))
                    .collect();
                commands::var::set(&conn, &experiment, &controls, &independents)
            }
            VarCommands::List { experiment } => {
                commands::var::list(&conn, &experiment)
            }
            VarCommands::Rm { experiment, name } => {
                commands::var::rm(&conn, &experiment, &name)
            }
        },
        Commands::Run { command } => match command {
            RunCommands::Start { experiment, vars } => {
                let parsed_vars: Vec<(String, String)> = vars
                    .iter()
                    .filter_map(|s| parse_key_value(s))
                    .collect();
                commands::run::start(&conn, &experiment, &parsed_vars)
            }
            RunCommands::Record { run_id, output } => {
                commands::run::record(&conn, &run_id, &output)
            }
            RunCommands::Fail { run_id, reason } => {
                commands::run::fail(&conn, &run_id, reason.as_deref())
            }
            RunCommands::Comment { run_id, text } => {
                commands::run::comment(&conn, &run_id, &text)
            }
            RunCommands::Artifact { run_id, file } => {
                commands::run::artifact(&conn, &run_id, &file)
            }
            RunCommands::List { experiment } => {
                commands::run::list(&conn, &experiment)
            }
            RunCommands::Show { run_id } => {
                commands::run::show(&conn, &run_id)
            }
        },
        Commands::Compare { experiment, sort_by, desc, group_by, where_clauses, cols, format } => {
            commands::compare::run(
                &conn,
                &experiment,
                sort_by.as_deref(),
                desc,
                group_by.as_deref(),
                &where_clauses,
                cols.as_deref(),
                &format,
            )
        }
        Commands::Export { experiment, format } => {
            commands::export::run(&conn, &experiment, &format)
        }
        Commands::Comment { experiment, text } => {
            commands::comment::add(&conn, &experiment, &text)
        }
        Commands::Comments { experiment } => {
            commands::comment::list(&conn, &experiment)
        }
        Commands::Describe { experiment, format } => {
            commands::describe::run(&conn, &experiment, &format)
        }
        Commands::Plan { experiment, shell } => {
            commands::plan::run(&conn, &experiment, &shell)
        }
        // Guide and Templates handled above
        Commands::Guide { .. } | Commands::Templates { .. } => unreachable!(),
    }
}
