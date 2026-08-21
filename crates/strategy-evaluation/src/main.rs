//! Write the offline calibration-v1 report from committed fixture data.

use std::{env, fs};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = env::args().nth(1).unwrap_or_else(|| {
        "crates/strategy-evaluation/data/generated/calibration-v1.report.json".to_owned()
    });
    let report = strategy_evaluation::evaluate_fixture()?;
    fs::write(&output, strategy_evaluation::report_json(&report)?)?;
    println!("wrote {output}");
    Ok(())
}
