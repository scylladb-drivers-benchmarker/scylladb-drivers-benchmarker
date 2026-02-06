//! This module supports the execution and measuring of the benchmarked code
//! from arbitrary commands. It performs no validation of those commands
//! eg. whether the run command they actually interacts with the output of the build command.

use std::str::FromStr;
use std::time::Duration;

use crate::command::{Command, CommandParsingError, PrintableOutput};
use crate::database::utilities::BenchmarkRecord;
use crate::utilities::BenchmarkPoint;

pub(crate) mod command_executor;
pub(crate) mod flame_executor;
pub(crate) mod output_executor;

#[justerror::Error(desc = "compilation failed")]
pub enum CompileError {
    CommandParsing(#[from] CommandParsingError),
    CompilationStarting(#[from] std::io::Error),
    CompilationRunning(PrintableOutput),
}

pub(crate) fn build_source(build_command: &str) -> Result<(), CompileError> {
    let command = Command::from_str(build_command)?;
    let mut command = command.process();

    let output = command.output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(CompileError::CompilationRunning(output.into()))
    }
}

/// The executor collects data according to its internals (time, perf, ...)
pub(crate) trait MeasuringEquipment {
    fn execute(
        &self,
        point: BenchmarkPoint,
    ) -> Result<BenchmarkRecord, Box<dyn std::error::Error + 'static>>;

    fn execute_with_timeout(
        &self,
        point: BenchmarkPoint,
        timeout: Duration,
    ) -> Result<BenchmarkRecord, Box<dyn std::error::Error + 'static>>;
}
