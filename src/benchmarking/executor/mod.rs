//! This module supports the execution and measuring of the benchmarked code
//! from arbitrary commands. It performs no validation of those commands
//! eg. whether the run command they actually interacts with the output of the build command.

use std::time::Duration;

use log::trace;

use crate::command::{Command, CommandParsingError, OutputWithTimeout, PrintableOutput};
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

pub(crate) fn build_source(command: Command) -> Result<(), CompileError> {
    let mut command = command.process();

    let output = command.output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(CompileError::CompilationRunning(output.into()))
    }
}

#[justerror::Error(desc = "command execution failed")]
pub enum PlainCommandError {
    Parsing(#[from] CommandParsingError),
    Spawning(#[from] std::io::Error),
    #[error(desc = "the command timed out")]
    TimedOut,
    Failed(PrintableOutput),
}

/// Runs an unmeasured command (env script, prepare/teardown phase),
/// failing on a non-zero exit status.
pub(crate) fn run_plain_command(
    command: Command,
    timeout: Option<Duration>,
) -> Result<(), PlainCommandError> {
    trace!("Executing: {command}");
    let mut process = command.process();
    let output = match timeout {
        Some(t) => process
            .output_with_timeout(t)?
            .ok_or(PlainCommandError::TimedOut)?,
        None => process.output()?,
    };
    if output.status.success() {
        Ok(())
    } else {
        Err(PlainCommandError::Failed(output.into()))
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
