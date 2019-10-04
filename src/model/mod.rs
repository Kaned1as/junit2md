use serde_derive::Deserialize;

#[derive(Debug, Deserialize)]
pub struct JunitReport {
    pub duration: Option<f64>,

    #[serde(rename = "testsuite", default)]
    pub testsuites: Vec<TestSuite>,
}

#[derive(Debug, Deserialize)]
pub struct TestSuite {
    pub name: String,
    pub tests: u64,
    pub id: Option<String>,
    pub package: Option<String>,
    pub failures: Option<u64>,
    pub disabled: Option<u64>,
    pub skipped: Option<u64>,
    pub errors: Option<u64>,
    pub time: Option<String>,
    pub timestamp: Option<String>,
    pub hostname: Option<String>,

    /// Properties of a certain test suite, common for all tests inside
    pub properties: Option<TestProperties>,

    #[serde(flatten)]
    pub outputs: TestOutputs,

    /// Test cases that this test suite consists of
    #[serde(rename = "testcase", default)]
    pub testcases: Vec<TestCase>,
}

#[derive(Debug, Deserialize)]
pub struct TestOutputs {
    #[serde(rename = "system-out", default)]
    pub system_out: Option<String>,

    #[serde(rename = "system-err", default)]
    pub system_err: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TestProperties {
    #[serde(rename = "property", default)]
    pub properties: Vec<TestProperty>,
}

#[derive(Debug, Deserialize)]
pub struct TestProperty {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct TestCase {
    pub name: String,
    pub assertions: Option<String>,
    pub time: Option<String>,
    pub classname: Option<String>,
    pub status: Option<String>,

    #[serde(flatten)]
    pub outputs: Option<TestOutputs>,

    pub skipped: Option<TestNegativeResult>,

    #[serde(rename = "error", default)]
    pub errors: Vec<TestNegativeResult>,

    #[serde(rename = "failure", default)]
    pub failures: Vec<TestNegativeResult>,
}

#[derive(Debug, Deserialize)]
pub struct TestNegativeResult {
    #[serde(rename = "type", default)]
    pub error_type: Option<String>,
    pub message: Option<String>,

    #[serde(rename = "$value")]
    pub body: Option<String>,
}
