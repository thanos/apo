//! JSON report writer.

use std::path::Path;

use crate::error::Result;
use crate::report::Report;

pub fn write_json(report: &Report, path: &Path) -> Result<()> {
    let file = std::fs::File::create(path)?;
    serde_json::to_writer_pretty(file, report)?;
    Ok(())
}

/// Serialize a report to a pretty JSON string.
pub fn to_string(report: &Report) -> Result<String> {
    Ok(serde_json::to_string_pretty(report)?)
}
