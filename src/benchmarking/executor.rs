//! This module supports the execution and measuring of the benchmarked code
//! from arbitrary commands. It performs little to no validation of those
//! commands eg. whether the run command they actually interacts with the
//! output of the build command.

use std::io;
use std::process::Output;
use std::str::FromStr;
use std::time::Duration;

use enum_dispatch::enum_dispatch;

use crate::command::{Command, CommandParsingError, OutputWithTimeout, PrintableOutput};
use crate::database::utilities::BenchmarkRecord;
use crate::measurement::MeasurementMethod;
use crate::utilities::BenchmarkPoint;
use crate::{cmd, command};

/// This is a token proving that the code being executed was compiled earlier.
/// Getting this from outside of this module happens only by invoking `build_source`.
#[must_use]
pub struct BuiltSource(());

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
pub fn build_source(build_command: &str) -> Result<BuiltSource, CompileError> {
    let command = Command::from_str(build_command)?;
    let mut command = command.process();

    let output = command.output()?;
    if output.status.success() {
        Ok(BuiltSource::new_unchecked())
    } else {
        Err(CompileError::CompilationRunning(output.into()))
    }
}

#[justerror::Error(desc = "measuring failed")]
pub enum MeasurementError {
    #[error(fmt = debug)]
    ExecutionFailed(Output),
    /// I named it such as no documentation is provided for how and when
    /// this error is thrown by Command.output.
    RustFailed(#[from] io::Error),
    WrongOutputFormat(#[from] std::string::FromUtf8Error),
}

/// The executor collects data according to its internals (time, perf, ...)
#[enum_dispatch(Executor)]
pub trait MeasuringEquipment {
    fn execute(&self, point: BenchmarkPoint) -> Result<BenchmarkRecord, MeasurementError>;
    fn execute_with_timeout(
        &self,
        point: BenchmarkPoint,
        timeout: Duration,
    ) -> Result<BenchmarkRecord, MeasurementError>;
}

/// Currently command is the only executor, once a more complicated one
/// is needed (eg. for Flamegraph) it should be added here
#[enum_dispatch]
#[derive(Debug)]
pub(crate) enum Executor {
    CommandExecutor,
}

impl Executor {
    pub fn new(
        _: BuiltSource,
        measurement_method: MeasurementMethod,
        run_command: command::Command,
    ) -> Self {
        match measurement_method {
            MeasurementMethod::Time => {
                CommandExecutor::new(cmd!("time", "-f", "%e"), run_command).into()
            }
            MeasurementMethod::Perf => {
                CommandExecutor::new(cmd!("perf", "stat", "--json"), run_command).into()
            }
            MeasurementMethod::Flamegraph => todo!(),
            MeasurementMethod::Command(_) => todo!(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct CommandExecutor {
    command: command::Command,
}

impl CommandExecutor {
    fn new(command: command::Command, run_command: command::Command) -> Self {
        CommandExecutor {
            command: command.with_cmd_arg(run_command),
        }
    }
}

impl CommandExecutor {
    fn handle_output(output: Output) -> Result<BenchmarkRecord, MeasurementError> {
        if output.status.success() {
            let str_stdout = String::from_utf8(output.stdout)?;
            let str_stderr = String::from_utf8(output.stderr)?;
            Ok(BenchmarkRecord::Data(
                (str_stdout + &str_stderr).trim_end().to_string(),
            ))
        } else {
            Err(MeasurementError::ExecutionFailed(output))
        }
    }
}

impl MeasuringEquipment for CommandExecutor {
    fn execute(&self, point: BenchmarkPoint) -> Result<BenchmarkRecord, MeasurementError> {
        Self::handle_output(
            self.command
                .clone()
                .with_arg(point.to_string())
                .process()
                .output()?,
        )
    }

    fn execute_with_timeout(
        &self,
        point: BenchmarkPoint,
        timeout: Duration,
    ) -> Result<BenchmarkRecord, MeasurementError> {
        self.command
            .clone()
            .with_arg(point.to_string())
            .process()
            .output_with_timeout(timeout)?
            .map(Self::handle_output)
            .unwrap_or(Ok(BenchmarkRecord::Timeout))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_execution_error() {
        let executor = Executor::new(
            BuiltSource::new_unchecked(),
            MeasurementMethod::Time,
            cmd!("git", "fail"),
        );
        let error = executor.execute(0).unwrap_err();
        assert!(matches!(error, MeasurementError::ExecutionFailed(_)));
    }

    #[test]
    fn test_execution_timeout() {
        let executor = Executor::new(
            BuiltSource::new_unchecked(),
            MeasurementMethod::Time,
            command::Command::from_str("sleep").unwrap(),
        );
        let output = executor
            .execute_with_timeout(2, std::time::Duration::from_secs(1))
            .unwrap();
        assert!(output.is_timeout());
    }
    #[test]
    fn test_execution_in_time() {
        let executor = Executor::new(
            BuiltSource::new_unchecked(),
            MeasurementMethod::Time,
            command::Command::from_str("sleep").unwrap(),
        );
        let output = executor
            .execute_with_timeout(1, std::time::Duration::from_secs(2))
            .unwrap();
        assert!(!output.is_timeout());
    }
}
