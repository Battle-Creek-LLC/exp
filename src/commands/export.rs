use anyhow::Result;
use rusqlite::Connection;

use crate::commands::compare;

pub fn run(conn: &Connection, experiment: &str, format: &str) -> Result<()> {
    // Export is compare with all data and no filters, in the requested format
    compare::run(conn, experiment, None, false, None, &[], None, format)
}
