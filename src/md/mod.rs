
use std::cmp;
use std::fmt::Display;

pub(super) fn create_h1(md: &mut String, title: &str) {
    create_header(md, "=", title);
}

pub(super) fn create_h2(md: &mut String, title: &str) {
    create_header(md, "-", title);
}

fn create_header(md: &mut String, underline: &str, title: &str) {
    md.push('\n');
    md.push_str(title);
    md.push('\n');
    md.push_str(&underline.repeat(title.chars().count()));
    md.push('\n');
}

pub(super) fn create_md_table(md: &mut String, rows: Vec<Vec<Box<dyn Display>>>) {
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
            md.push_str(&pad_cell_text(&header_name, column_widths[index]));
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
                let padded_cell_text = pad_cell_text(&cell_text, column_widths[index]);
                md.push_str(&padded_cell_text);
                md.push('|');
            }
            md.push('\n');
        }
        md.push('\n');
    }
}

/// Pads cell text from right and left so it looks centered inside the table cell
/// 
/// `column_width` - precomputed column width to compute padding length from
pub(super) fn pad_cell_text(content: &str, column_width: usize) -> String {
    let mut result = String::new();
    if content.len() > 0 {
        // have header at specified position
        // compute difference between width and text length
        let len_diff = column_width - content.chars().count();
        if len_diff > 0 {
            // should pad
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