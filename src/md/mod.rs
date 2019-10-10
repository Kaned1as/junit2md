
use std::cmp;
use std::fmt::Display;

/// Creates main header in Markdown
pub(super) fn create_h1(md: &mut String, title: &str) {
    create_header(md, "=", title);
}

/// Creates secondary header in Markdown
pub(super) fn create_h2(md: &mut String, title: &str) {
    create_header(md, "-", title);
}

/// Creates auxiliary header in Markdown
pub(super) fn create_h3(md: &mut String, title: &str) {
    md.push('\n');
    md.push_str(&format!("### {} ###", title));
    md.push('\n');
}

/// Helper function to create different types of headers
fn create_header(md: &mut String, underline: &str, title: &str) {
    md.push('\n');
    md.push_str(title);
    md.push('\n');
    md.push_str(&underline.repeat(title.chars().count()));
    md.push('\n');
}

/// Creates spoiler tag in Markdown (GFM)
pub(super) fn create_code_detail(md: &mut String, summary: &str, code: &str) {
    md.push_str("<details>\n");
    md.push_str(&format!("    <summary>{}</summary>\n", summary));
    md.push_str("\n");
    md.push_str(&tabulate(&code, "    "));
    md.push_str("\n");
    md.push_str("</details>\n");
    md.push('\n');
}

/// Appends a number of spaces before each newline
fn tabulate(input: &str, to_prepend: &str) -> String {
    let mut result = input.to_owned();
    result.insert_str(0, to_prepend); // insert at the beginning
    return result.replace('\n', &format!("\n{}", to_prepend)); // insert after each newline
}

/// Creates table in Markdown. Table is passed as a vector of rows, top-to-down, each row is a vector of cells, left-to-right.
pub(super) fn create_md_table(md: &mut String, rows: Vec<Vec<Box<dyn Display>>>, align_left_first_column: bool) {
    if rows.len() < 2 {
        // we need at least one header row and one value row
        return;
    }

    // first, detect column width for each row
    let column_count = rows[0].len();
    let mut column_widths = vec![3; column_count];

    // detect max column width
    for row in rows.iter() {
        for index in 0..column_count {
            // from regular rows
            if let Some(cell) = row.get(index) {
                let text = cell.to_string();
                column_widths[index] = cmp::max(column_widths[index], text.len());
            }
        }
    }
    
    if let Some((headers, data)) = rows.split_first() {
        // make headers
        md.push('|');
        for index in 0..column_count {
            let header_name = headers[index].to_string();
            md.push_str(&pad_cell_text(&header_name, column_widths[index], true));
            md.push('|');
        }
        md.push('\n');

        // make header-divider row
        md.push('|');
        for index in 0..column_count {
            let width = column_widths[index];
            md.push_str(&"-".repeat(width));
            md.push('|');
        }
        md.push('\n');

        for row in data.iter() {
            md.push('|');
            for index in 0..column_count {
                let cell_text = row[index].to_string();
                if align_left_first_column && index == 0 {
                    let padded_right_text = pad_cell_text(&cell_text, column_widths[index], false);
                    md.push_str(&padded_right_text);
                } else {
                    let padded_text = pad_cell_text(&cell_text, column_widths[index], true);
                    md.push_str(&padded_text);
                }
                
                md.push('|');
            }
            md.push('\n');
        }
        md.push('\n');
    }
}

/// Pads Markdown cell text so it looks aligned in the table. Not necessary but makes raw Markdown more readable.
pub(super) fn pad_cell_text(content: &str, column_width: usize, align_center: bool) -> String {
    let mut result = String::new();
    if content.len() > 0 {
        // have header at specified position
        // compute difference between width and text length
        let len_diff = column_width - content.chars().count();
        if len_diff > 0 {
            // should pad
            if !align_center {
                result.push_str(&content);
                result.push_str(&" ".repeat(len_diff));
                return result;
            }

            if len_diff > 1 {
                // should pad from both sides
                let pad_len = len_diff / 2;
                let remainder = len_diff % 2;
                result.push_str(&" ".repeat(pad_len));
                result.push_str(&content);
                result.push_str(&" ".repeat(pad_len + remainder));
            } else {
                // it's just one space, add at the end
                result.push_str(&content);
                result.push(' ');
            }
        } else {
            // shouldn't pad, text fills whole cell
            result.push_str(&content);
        }
    } else {
        // no text in this cell, fill cell with spaces
        let pad_len = column_width;
        result.push_str(&" ".repeat(pad_len));
    }

    return result;
}