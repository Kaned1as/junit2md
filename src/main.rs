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

use lang_specific::*;
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
                let md = suites_to_md_mult(mult.testsuites);
                println!("{}", md);
                return;
            }
        }

        // not an aggregated report, deserialize into singular
        let singular: Result<TestSuite, XmlError> = from_reader(junit_content.as_bytes());
        if singular.is_ok() {
            // that's real singular testcase, report it
            let md = suite_to_md_single(singular.unwrap());
            println!("{}", md);
            return;
        } else {
            eprintln!("Couldn't parse JUnit XML as singular: {}", singular.unwrap_err());
            return;
        }
    }

    // there are multiple files, report them as aggregated
    let mut testsuites: Vec<TestSuite> = vec![];
    for junit_file in junit_files {
        // it must be a single file
        let junit_content = fs::read_to_string(junit_file).expect(&format!("Can't read JUnit file {}", junit_file));
        let singular: Result<TestSuite, XmlError> = from_reader(junit_content.as_bytes());
        if singular.is_ok() {
            testsuites.push(singular.unwrap());
        } else {
            eprintln!("Couldn't parse JUnit XML {} as singular: {}", junit_file, singular.unwrap_err());
        }
    }

    // now post an aggregated report
    let md = suites_to_md_mult(testsuites);
    println!("{}", md);
}

/// Converts multiple suites to markdown, consuming them. 
/// Only prints totals for each test suite and only reports failed test cases in the overview.
/// 
/// Arguments:
/// * `suites` - test suites to report.
fn suites_to_md_mult(suites: Vec<TestSuite>) -> String {
    let mut md = String::new();

    create_h1(&mut md, "Aggregated test report");
    add_totals_multiple(&mut md, &suites);

    let failed_tests: Vec<TestCase> = suites.into_iter()
                             .map(|suite| suite.testcases)
                             .flatten()
                             .filter(|test| test.skipped.is_some() || !test.failures.is_empty() || !test.errors.is_empty())
                             .collect();
                             
    add_testcases_fail_details(&mut md, &failed_tests);

    return md;
}

/// Converts single suite to markdown, consuming it. 
/// Prints totals for the suite, status for every test case and reports failed tests in overview.
/// 
/// Arguments:
/// * `suite` - test suite to report
fn suite_to_md_single(suite: TestSuite) -> String {
    let mut md = String::new();

    create_h1(&mut md, omit_java_package(&suite.name));
    add_suite_properties(&mut md, &suite);
    add_totals_singular(&mut md, &suite);
    add_testcases_summary(&mut md, &suite);
    add_testcases_fail_details(&mut md, &suite.testcases);

    return md;
}

/// Adds suite properties section to the report.
/// There can be lots of them so it only does so if `IS_VERBOSE` flag is set.
/// 
/// Arguments:
/// * `md` - the report to add properties section to.
/// * `suite` - test suite to get properties from.
fn add_suite_properties(md: &mut String, suite: &TestSuite) {
    if !IS_VERBOSE.load(Ordering::Relaxed) {
        // not needed in verbose mode, bail out
        return;
    }

    // verbose mode is on, report all the details
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

/// Adds summary table for testcases.
/// Each test is reported and failing tests have a link to see their details.
/// 
/// Arguments:
/// * `md` - the report to add testcase summary section to.
/// * `suite` - test suite to get tests.
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

    // iterate over each test case and add a row with the description to the table
    let mut fail_index = 0;
    for test in tests {
        let name = omit_java_package(&test.name).to_owned();
        let test_time = test.time.to_owned().unwrap_or_default();

        if !test.errors.is_empty() {
            // this is a test with error
            table.push(vec![
                Box::new(name),
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
                Box::new(name),
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
                Box::new(name),
                Box::new("✂"), 
                Box::new(test_time),
                Box::new(format!("[[{0}]](#c-{0})", fail_index))
            ]);
            fail_index += 1;
            continue;
        }

        // this is a successful test
        table.push(vec![
            Box::new(name),
            Box::new("✓"), 
            Box::new(test_time),
            Box::new(""),
        ]);
    }
    create_md_table(md, table, true);
}

/// Adds summary table for a single testsuite.
/// Number of tests for each result is reported.
/// 
/// Arguments:
/// * `md` - the report to add testcase summary section to.
/// * `suite` - test suite to get tests.
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
        Box::new("**Success**"),
        Box::new(success_tests),
        Box::new(success_tests * 100 / suite.tests)
    ]);

    create_md_table(md, table, false);
}

/// Adds details for failed testcases.
/// Each testcase is reported along with its output and content of failure.
/// 
/// Arguments:
/// * `md` - the report to add testcase summary section to.
/// * `tests` - tests that should be reported. Successful ones are skipped.
fn add_testcases_fail_details(md: &mut String, tests: &Vec<TestCase>) {
    // no failures to report
    if !tests.iter().any(|test| test.skipped.is_some() || !test.failures.is_empty() || !test.errors.is_empty()) {
        return;
    }

    create_h2(md, "Failures");

    let mut fail_index = 0;
    for test in tests {
        if !test.errors.is_empty() {
            let error = &test.errors[0];

            // this is a test with error
            report_negative_result(md, fail_index, test, error);
            fail_index += 1;
            continue;
        }

        if !test.failures.is_empty() {
            let failure = &test.failures[0];

            // this is a test with failure
            report_negative_result(md, fail_index, test, failure);
            fail_index += 1;
            continue;
        }

        if let Some(skipped_desc) = &test.skipped {
            // this is a skipped test
            report_negative_result(md, fail_index, test, skipped_desc);
            fail_index += 1;
            continue;
        }
    }
}

/// Helper function that formats a failure result in a human-readable way.
/// Basically it wraps long content in stdout/stderr and failure bodies into spoilers
/// that can be expanded by user.
/// 
/// Arguments:
/// * `md` - the report to add testcase summary section to.
/// * `fail_index` - index of anchor to use. Testcase tables may be referring to this.
/// * `test` - testcase to report.
/// * `result` - negative result to report.
fn report_negative_result(md: &mut String, fail_index: usize, test: &TestCase, result: &TestNegativeResult) {
    let not_specified = String::from("Not specified");

    md.push_str(&format!("<a id=\"c-{}\"/>\n\n", fail_index));
    create_h3(md, &test.name);
    md.push('\n');

    if let Some(classname) = &test.classname {
        let classname_simple = omit_java_package(classname);
        md.push_str(&format!("* Classname: {}\n", classname_simple));
    }

    let failure_message = result.message.as_ref().unwrap_or(&not_specified);
    md.push_str(&format!("* Fail reason: `{}`\n", failure_message));

    if let Some(body) = &result.body {
        create_code_detail(md, "Click to show details", &body);
    }

    if !IS_VERBOSE.load(Ordering::Relaxed) {
        // not verbose, skip stdout/stderr
        return;
    }

    if let Some(out) = &test.system_out {
        create_code_detail(md, "Click to show test stdout", &out);
    }

    if let Some(err) = &test.system_err {
        create_code_detail(md, "Click to show test stderr", &err);
    }
}

/// Adds summary table for multiple testsuites.
/// Only numbers of successful/failed/total tests are reported.
/// 
/// Arguments:
/// * `md` - the report to add testcase summary section to.
/// * `suites` - test suites to get info from.
fn add_totals_multiple(md: &mut String, suites: &Vec<TestSuite>) {
    md.push('\n');

    let mut table: Vec<Vec<Box<dyn Display>>> = vec![];
    table.push(vec![
        Box::new("Suite name"),
        Box::new("Time taken, s"),
        Box::new("Success"),
        Box::new("Skipped"),
        Box::new("Disabled"),
        Box::new("Failures"),
        Box::new("Total")
    ]);


    let mut skipped_total = 0;
    let mut disabled_total = 0;
    let mut failed_total = 0;
    let mut success_total = 0;
    let mut overall_total = 0;
    for suite in suites {
        let name = omit_java_package(&suite.name).to_owned();
        let time = suite.time.as_ref().unwrap_or(&String::new()).to_owned();

        let skipped_tests = suite.skipped.unwrap_or(0);
        let disabled_tests = suite.disabled.unwrap_or(0);
        let failed_tests = suite.failures.unwrap_or(0) + suite.errors.unwrap_or(0);
        let success_tests = suite.tests - failed_tests - disabled_tests - skipped_tests;

        if skipped_tests > 0 {
            
        }

        table.push(vec![
            Box::new(name),
            Box::new(time), 
            Box::new(success_tests), 
            Box::new(skipped_tests), 
            Box::new(disabled_tests), 
            Box::new(failed_tests), 
            Box::new(suite.tests)
        ]);

        skipped_total += skipped_tests;
        disabled_total += disabled_tests;
        failed_total += failed_tests;
        success_total += success_tests;
        overall_total += suite.tests;
    }

    table.push(vec![
        Box::new("**Total**"),
        Box::new("N/A"), 
        Box::new(success_total), 
        Box::new(skipped_total), 
        Box::new(disabled_total), 
        Box::new(failed_total), 
        Box::new(overall_total)
    ]);

    create_md_table(md, table, true);
}