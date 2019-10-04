mod model;
mod md;

use std::fs;
use std::fmt::Display;

use clap::{Arg, App};
use serde_xml_rs::from_reader;
use serde_xml_rs::Error as XmlError;

use model::*;
use md::*;

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

    create_h1(&mut md, &suite.name);
    add_suite_properties(&mut md, &suite);
    add_totals_singular(&mut md, &suite);
    add_testcases_summary(&mut md, &suite);
    add_testcases_fail_details(&mut md, &suite);

    return md;
}

fn add_suite_properties(md: &mut String, suite: &TestSuite) {
    if suite.timestamp.is_some() && suite.hostname.is_some() && suite.time.is_some() {
        md.push('\n');
        md.push_str(&format!("Testset was started on host {hostname} at {timestamp} and took {time} seconds to finish.", 
            hostname=suite.hostname.as_ref().unwrap(), 
            timestamp=suite.timestamp.as_ref().unwrap(), 
            time=suite.time.as_ref().unwrap())
        );
        md.push('\n');
    }

    if suite.properties.is_none() {
        return;
    }
    
    md.push('\n');
    md.push_str("Properties:");

    let desc = suite.properties.as_ref().unwrap();
    for prop in &desc.properties {
        md.push('\n');
        md.push_str(&format!("* {name}: {value}", name=prop.name, value=prop.value));
    }
    md.push('\n');
}

fn add_testcases_summary(md: &mut String, suite: &TestSuite) {
    create_h2(md, "Breakdown by testcases");

    let tests = &suite.testcases;
    let mut table: Vec<Vec<Box<dyn Display>>> = vec![];
    table.push(vec![
        Box::new("Testcase name"),
        Box::new("Status"), 
        Box::new("Time"),
        Box::new("Cause"),
    ]);

    let non_applicable = String::from("N/A");
    let non_specified = String::from("Not specified");
    let mut fail_index = 0;
    for test in tests {
        let test_time = test.time.to_owned().unwrap_or_default();

        if !test.errors.is_empty() {
            // this is a test with error
            let error_message = test.errors[0].message.as_ref().unwrap_or(&non_applicable);
            let first_error_line = error_message.lines().next().unwrap().to_owned();
            table.push(vec![
                Box::new(test.name.to_owned()),
                Box::new("‼"), 
                Box::new(test_time),
                Box::new(format!("[[{0}]](#c-{0})", fail_index))
            ]);
            fail_index += 1;
            continue;
        }

        if !test.failures.is_empty() {
            // this is a test with failure
            let failure_message = test.failures[0].message.as_ref().unwrap_or(&non_applicable);
            let first_failure_line = failure_message.lines().next().unwrap().to_owned();
            table.push(vec![
                Box::new(test.name.to_owned()),
                Box::new("✗"), 
                Box::new(test_time),
                Box::new(format!("[[{0}]](#c-{0})", fail_index))
            ]);
            fail_index += 1;
            continue;
        }

        if let Some(skipped_desc) = &test.skipped {
            // this is a skipped test
            let skip_message = skipped_desc.message.as_ref().unwrap_or(&non_specified);
            let first_skip_line = skip_message.lines().next().unwrap().to_owned();
            table.push(vec![
                Box::new(test.name.to_owned()),
                Box::new("✂"), 
                Box::new(test_time),
                Box::new(format!("[[{0}]](#cause-{0})", fail_index))
            ]);
            fail_index += 1;
            continue;
        }

        // this is a successful test
        table.push(vec![
            Box::new(test.name.to_owned()),
            Box::new("✓"), 
            Box::new(test_time),
            Box::new(""),
        ]);
        continue;
    }
    create_md_table(md, table);
}

fn add_totals_singular(md: &mut String, suite: &TestSuite) {
    create_h2(md, "Overall status");

    let mut table: Vec<Vec<Box<dyn Display>>> = vec![];
    table.push(vec![
        Box::new("Type"),
        Box::new("Number of tests"),
        Box::new("% of total")
    ]);

    let skipped_tests = suite.skipped.unwrap_or(0);
    table.push(vec![
        Box::new("Skipped"),
        Box::new(skipped_tests),
        Box::new(skipped_tests * 100 / suite.tests)
    ]);

    let disabled_tests = suite.disabled.unwrap_or(0);
    table.push(vec![
        Box::new("Disabled"),
        Box::new(disabled_tests),
        Box::new(disabled_tests * 100 / suite.tests)
    ]);

    let failed_tests = suite.failures.unwrap_or(0) + suite.errors.unwrap_or(0);
    table.push(vec![
        Box::new("Failed"),
        Box::new(failed_tests),
        Box::new(failed_tests * 100 / suite.tests)
    ]);

    let success_tests = suite.tests - failed_tests - disabled_tests - skipped_tests;
    table.push(vec![
        Box::new("*Successful*"),
        Box::new(success_tests),
        Box::new(success_tests * 100 / suite.tests)
    ]);

    create_md_table(md, table);
}

fn add_testcases_fail_details(md: &mut String, suite: &TestSuite) {
    let tests = &suite.testcases;

    // no failures to report
    if !tests.iter().any(|test| test.skipped.is_some() || !test.failures.is_empty() || !test.errors.is_empty()) {
        return;
    }

    create_h2(md, "Failures");
    for test in tests {
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