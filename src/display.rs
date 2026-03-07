use comfy_table::{ContentArrangement, Table};

pub fn build_table(headers: &[&str], rows: &[Vec<String>]) -> Table {
    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(headers.iter().map(|h| h.to_string()));

    for row in rows {
        table.add_row(row.iter().map(|c| c.to_string()));
    }

    table
}

