//! This module supports the execution and measuring of the benchmarked code
//! from arbitrary commands. It performs little to no validation of those
//! commands eg. whether the run command they actually interacts with the
//! output of the build command.

use std::error::Error;
use std::io;
use std::process::Output;
use std::str::FromStr;
use std::time::Duration;

use crate::benchmarking::BenchMeasure;
use crate::benchmarking::executor::command_executor::CommandExecutor;
use crate::benchmarking::executor::flame_executor::FlameExecutor;
use crate::command;
use crate::command::{Command, CommandParsingError, PrintableOutput};
use crate::database::utilities::BenchmarkRecord;
use crate::utilities::BenchmarkPoint;

mod command_executor;
mod flame_executor;

/// This is a token proving that the code being executed was compiled earlier.
/// Getting this from outside of this module happens only by invoking `build_source`.
#[must_use]
pub(crate) struct BuiltSource(());

impl BuiltSource {
    fn new_unchecked() -> Self {
        BuiltSource(())
    }
}

#[justerror::Error(desc = "compilation failed")]
pub enum CompileError {
    CommandParsing(#[from] CommandParsingError),
    CompilationStarting(#[from] std::io::Error),
    CompilationRunning(PrintableOutput),
}

/// Builds the source code, using the provided command.
/// This is the only way to receive `BuiltSource` from outside.
pub(crate) fn build_source(build_command: &str) -> Result<BuiltSource, CompileError> {
    let command = Command::from_str(build_command)?;
    let mut command = command.process();

    let output = command.output()?;
    if output.status.success() {
        Ok(BuiltSource::new_unchecked())
    } else {
        Err(CompileError::CompilationRunning(output.into()))
    }
}

/// The executor collects data according to its internals (time, perf, ...)
pub(crate) trait MeasuringEquipment {
    type MeasurementError: Error + 'static;

    fn execute(&self, point: BenchmarkPoint) -> Result<BenchmarkRecord, Self::MeasurementError>;
    fn execute_with_timeout(
        &self,
        point: BenchmarkPoint,
        timeout: Duration,
    ) -> Result<BenchmarkRecord, Self::MeasurementError>;
}

pub(crate) trait Callback {
    type ReturnType;

    fn call(self, value: impl MeasuringEquipment) -> Self::ReturnType;
}

pub(crate) fn execute_all<CallbackType: Callback>(
    _: BuiltSource,
    measurement_method: BenchMeasure,
    run_command: command::Command,
    callback: CallbackType,
) -> CallbackType::ReturnType {
    match measurement_method {
        BenchMeasure::Time => callback.call(CommandExecutor::new_time(run_command)),
        BenchMeasure::PerfStat => callback.call(CommandExecutor::new_perf(run_command)),
        BenchMeasure::FlameGraph {
            flame_repo,
            frequency,
            store_dir,
        } => callback.call(FlameExecutor::new(
            flame_repo,
            store_dir,
            frequency,
            run_command,
        )),
        BenchMeasure::Command(command) => callback.call(CommandExecutor::new(command, run_command)),
    }
}

#[justerror::Error(desc = "measuring failed")]
pub(crate) enum CommandMeasurementError {
    #[error(fmt = debug)]
    ExecutionFailed(Output),
    /// I named it such as no documentation is provided for how and when
    /// this error is thrown by Command.output.
    RustFailed(#[from] io::Error),
    WrongOutputFormat(#[from] std::string::FromUtf8Error),
}
