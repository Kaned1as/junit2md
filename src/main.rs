mod model;
mod md;
mod lang_specific;

use std::fs;
use std::fmt::Display;

use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;

use clap::{Arg, App};
use serde_xml_rs::from_reader;
use serde_xml_rs::Error as XmlError;

use model::*;
use md::*;

static IS_VERBOSE: AtomicBool = AtomicBool::new(false);

fn main() {
    let cli_args = App::new("JUnit 2 Markdown converter")
                        .version("0.1.0")
                        .author("Oleg `Kanedias` Chernovskiy <kanedias@keemail.me>")
                        .about("Generates Markdown text from JUnit XML report")
                        .arg(Arg::with_name("input-files")
                                .multiple(true)
                                .required(true)
                                .help("Input JUnit XML(s) to generate Markdown from. \
                                       Generates verbose report in case there's single file. \
                                       Generates brief report in case there are multiple files or it's an aggregated report."))
                        .arg(Arg::with_name("verbose")
                                .short("v")
                                .required(false)
                                .help("Verbose output (hostnames, properties, standard streams)"))
                        .get_matches();

    IS_VERBOSE.store(cli_args.is_present("verbose"), Ordering::Relaxed);

    let mut junit_files = cli_args.values_of("input-files").unwrap();

    // Unfortunately, serde-xml-rs doesn't fully support enum
    // decoding (or maybe I couldn't get it to work).
    // Once it does, the following code should be rewritten
    // as enum JunitReport { Single(TestSuite), Multiple(TestSuiteSet) }

    if junit_files.len() == 1 {
        // it's a single file, let's try deserializing into aggregated report first
        let junit_content = fs::read_to_string(junit_files.next().unwrap()).expect("Can't read JUnit file");
        let mult: Result<JunitReport, XmlError> = from_reader(junit_content.as_bytes());
        if let Some(mult) = mult.ok() {
            if mult.testsuites.len() != 0 {
                // that's real mult testcase, report it
                println!("{:#?}", mult);
                return;
            }
        }

        // not an aggregated report, deserialize into singular
        let singular: Result<TestSuite, XmlError> = from_reader(junit_content.as_bytes());
        if singular.is_ok() {
            // that's real singular testcase, report it
            let md = suite_to_md(singular.unwrap());
            println!("{}", md);
            return;
        } else {
            eprintln!("Couldn't parse JUnit XML as singular: {}", singular.unwrap_err());
            return;
        }
    }

    // there are multiple files, report them as aggregated
    for junit_file in junit_files {

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
    if !IS_VERBOSE.load(Ordering::Relaxed) {
        return;
    }

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

    let mut fail_index = 0;
    for test in tests {
        let test_time = test.time.to_owned().unwrap_or_default();

        if !test.errors.is_empty() {
            // this is a test with error
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
            table.push(vec![
                Box::new(test.name.to_owned()),
                Box::new("✗"), 
                Box::new(test_time),
                Box::new(format!("[[{0}]](#c-{0})", fail_index))
            ]);
            fail_index += 1;
            continue;
        }

        if test.skipped.is_some() {
            // this is a skipped test
            table.push(vec![
                Box::new(test.name.to_owned()),
                Box::new("✂"), 
                Box::new(test_time),
                Box::new(format!("[[{0}]](#c-{0})", fail_index))
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

    let mut fail_index = 0;
    let not_specified = String::from("Not specified");
    for test in tests {

        if !test.errors.is_empty() {
            let error = &test.errors[0];

            // this is a test with error
            create_h3(md, &test.name);


            let error_message = error.message.as_ref().unwrap_or(&not_specified);
            md.push_str(&format!("<a id=\"c-{}\"/>  ", fail_index));
            md.push_str(&format!("Error reason: {}  \n", error_message));
            report_negative_result(md, test, error);
            fail_index += 1;
            continue;
        }

        if !test.failures.is_empty() {
            let failure = &test.failures[0];

            // this is a test with failure
            create_h3(md, &test.name);

            let failure_message = failure.message.as_ref().unwrap_or(&not_specified);
            md.push_str(&format!("<a id=\"c-{}\"/>  ", fail_index));
            md.push_str(&format!("Fail reason: {}  \n", failure_message));
            report_negative_result(md, test, failure);
            fail_index += 1;
            continue;
        }

        if let Some(skipped_desc) = &test.skipped {
            // this is a skipped test
            create_h3(md, &test.name);

            let skip_message = skipped_desc.message.as_ref().unwrap_or(&not_specified);
            md.push_str(&format!("<a id=\"c-{}\"/>  ", fail_index));
            md.push_str(&format!("Skip reason: {}  \n", skip_message));
            report_negative_result(md, test, skipped_desc);
            fail_index += 1;
            continue;
        }
    }
}

fn report_negative_result(md: &mut String, test: &TestCase, result: &TestNegativeResult) {
    if let Some(body) = &result.body {
        create_code_detail(md, "Click to show details", &body);
    }

    if !IS_VERBOSE.load(Ordering::Relaxed) {
        return;
    }

    if let Some(out) = &test.outputs.system_out {
        create_code_detail(md, "Click to show test stdout", &out);
    }

    if let Some(err) = &test.outputs.system_err {
        create_code_detail(md, "Click to show test stderr", &err);
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