pub mod model;

use std::fs;
use std::cmp;
use std::fmt::Display;

use clap::{Arg, App};
use serde_xml_rs::from_reader;
use serde_xml_rs::Error as XmlError;

use model::*;

fn main() {
    let cli_args = App::new("JUnit 2 Markdown converter")
                        .version("0.1.0")
                        .author("Oleg `Kanedias` Chernovskiy <kanedias@keemail.me>")
                        .about("Generates Markdown text from JUnit XML report")
                        .arg(Arg::with_name("INPUT")
                                .required(true)
                                .help("Input JUnit XML to generate Markdown from"))
                        .arg(Arg::with_name("verbose")
                                .short("v")
                                .required(false)
                                .help("Verbose output (hostnames, properties, standard streams)"))
                        .get_matches();

    let junit_file = cli_args.value_of("INPUT").unwrap();
    let junit_content = fs::read_to_string(junit_file).expect("Can't read JUnit file");

    // Unfortunately, serde-xml-rs doesn't fully support enum
    // decoding (or maybe I couldn't get it to work).
    // Once it does, the following code should be rewritten
    // as enum JunitReport { Single(TestSuite), Multiple(TestSuiteSet) }

    // Most test-cases are singular, those JUnit produces as TEST-full.class.name.xml
    let singular: Result<TestSuite, XmlError> = from_reader(junit_content.as_bytes());
    if singular.is_ok() {
        let md = suite_to_md(singular.unwrap());
        println!("{}", md);
        //println!("{:#?}", singular.unwrap());
        return;
    } else {
        eprintln!("Couldn't parse JUnit XML as singular: {}", singular.unwrap_err());
    }

    // Multiple test-cases are more common in Jenkins aggregated reports
    let mult: Result<JunitReport, XmlError> = from_reader(junit_content.as_bytes());
    if mult.is_ok() {
        let mult = mult.unwrap();
        if mult.testsuites.len() == 0 {
            // it got parsed to empty mult XML, this probably means it was just non-proper
            // singular test suite. Just return so the only thing user sees is singular error
            return;
        }

        // that's real mult testcase, parse it
        println!("{:#?}", mult);
    } else {
        eprintln!("Couldn't parse JUnit XML as mult: {}", mult.unwrap_err());
    }
}

fn suite_to_md(suite: TestSuite) -> String {
    let mut md = String::new();

    md.push_str(&suite.name);
    md.push('\n');
    md.push_str(&"=".repeat(suite.name.chars().count()));
    md.push('\n');

    add_totals(&mut md, &suite);
    add_testcases(&mut md, suite.testcases);

    return md;
}

fn add_testcases(md: &mut String, tests: Vec<TestCase>) {
    for test in tests {
        let test_time = test.time.unwrap_or_default();

        if !test.errors.is_empty() {
            // this is a test with error
            continue;
        }

        if !test.failures.is_empty() {
            // this is a test with failure
            continue;
        }

        if let Some(skipped_desc) = test.skipped {
            // this is a skipped test
            continue;
        }

        if let Some(outputs) = test.outputs {
            // this is a successful test
            continue;
        }
    }
    
}

fn add_totals(md: &mut String, suite: &TestSuite) {
    md.push('\n');

    let skipped_tests = suite.skipped.unwrap_or(0);
    let disabled_tests = suite.disabled.unwrap_or(0);
    let failed_tests = suite.failures.unwrap_or(0) + suite.errors.unwrap_or(0);
    let success_tests = suite.tests - failed_tests - disabled_tests - skipped_tests;

    let table_headers: Vec<Box<dyn Display>> = vec![
        Box::new("Success"), 
        Box::new("Skipped"), 
        Box::new("Disabled"), 
        Box::new("Failures"), 
        Box::new("Total")
    ];
    let table_row: Vec<Box<dyn Display>> = vec![
        Box::new(success_tests), 
        Box::new(skipped_tests), 
        Box::new(disabled_tests), 
        Box::new(failed_tests), 
        Box::new(suite.tests)
    ];
    create_md_table(md, vec![table_headers, table_row]);
}

fn create_md_table(md: &mut String, rows: Vec<Vec<Box<dyn Display>>>) {
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
fn pad_cell_text(content: &str, column_width: usize) -> String {
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