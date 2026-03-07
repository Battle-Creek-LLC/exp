use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct Experiment {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub template: Option<String>,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct Variable {
    pub id: String,
    pub exp_id: String,
    pub name: String,
    pub role: String,
    pub val_list: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct Run {
    pub id: String,
    pub exp_id: String,
    pub status: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub output: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct RunVariable {
    pub run_id: String,
    pub var_name: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct Comment {
    pub id: String,
    pub exp_id: Option<String>,
    pub run_id: Option<String>,
    pub body: String,
    pub added_at: String,
}
