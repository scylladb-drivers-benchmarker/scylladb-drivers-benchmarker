//! This module supports the execution and measuring of the benchmarked code
//! from arbitrary commands. It performs little to no validation of those
//! commands eg. whether the run command they actually interacts with the
//! output of the build command.

use std::error::Error;
use std::io;
use std::path::PathBuf;
use std::process::Output;
use std::str::FromStr;
use std::string::FromUtf8Error;
use std::time::Duration;

use subprocess::{CaptureData, Exec, Pipeline, PopenError};

use crate::command::{Command, CommandParsingError, OutputWithTimeout, PrintableOutput};
use crate::database::utilities::BenchmarkRecord;
use crate::measurement::MeasurementMethod;
use crate::plotting::error;
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

/// The executor collects data according to its internals (time, perf, ...)
pub trait MeasuringEquipment {
    type MeasurementError: Error;

    fn execute(&self, point: BenchmarkPoint) -> Result<BenchmarkRecord, Self::MeasurementError>;
    fn execute_with_timeout(
        &self,
        point: BenchmarkPoint,
        timeout: Duration,
    ) -> Result<BenchmarkRecord, Self::MeasurementError>;

    fn execute_all<CallbackError, ResultingError>(
        &self,
        points: impl Iterator<Item = BenchmarkPoint>,
        mut callback: impl FnMut(BenchmarkPoint, BenchmarkRecord) -> Result<(), CallbackError>,
        opt_timeout: Option<Duration>,
    ) -> Result<(), ResultingError>
    where
        CallbackError: Error,
        ResultingError: Error + From<CallbackError> + From<Self::MeasurementError>,
    {
        let execute = |point| {
            if let Some(timeout) = opt_timeout {
                self.execute_with_timeout(point, timeout)
            } else {
                self.execute(point)
            }
        };
        for point in points {
            let result = execute(point)?;
            callback(point, result)?;
        }
        Ok(())
    }
}

pub fn execute_all<CallbackError, ResultingError>(
    _: BuiltSource,
    measurement_method: MeasurementMethod,
    run_command: command::Command,
    points: impl Iterator<Item = BenchmarkPoint>,
    callback: impl FnMut(BenchmarkPoint, BenchmarkRecord) -> Result<(), CallbackError>,
    timeout: Option<Duration>,
) -> Result<(), ResultingError>
where
    CallbackError: Error,
    ResultingError: Error + From<CallbackError> + From<CommandMeasurementError>,
{
    match measurement_method {
        MeasurementMethod::Time => CommandExecutor::new_time(run_command),
        MeasurementMethod::Perf => CommandExecutor::new_perf(run_command),
        MeasurementMethod::Flamegraph(flame_path) => todo!(),
        MeasurementMethod::Command(command) => CommandExecutor::new(command, run_command),
    }
    .execute_all(points, callback, timeout)
}

#[justerror::Error(desc = "measuring failed")]
pub enum CommandMeasurementError {
    #[error(fmt = debug)]
    ExecutionFailed(Output),
    /// I named it such as no documentation is provided for how and when
    /// this error is thrown by Command.output.
    RustFailed(#[from] io::Error),
    WrongOutputFormat(#[from] std::string::FromUtf8Error),
}

#[derive(Debug)]
pub(crate) struct CommandExecutor(command::Command);

impl CommandExecutor {
    fn new(command: command::Command, run_command: command::Command) -> Self {
        CommandExecutor(command.with_cmd_arg(run_command))
    }

    fn new_time(run_command: command::Command) -> Self {
        CommandExecutor::new(cmd!("time", "-f", "%e"), run_command)
    }

    fn new_perf(run_command: command::Command) -> Self {
        CommandExecutor::new(cmd!("perf", "stat", "--json"), run_command)
    }
}

impl CommandExecutor {
    fn handle_output(output: Output) -> Result<BenchmarkRecord, CommandMeasurementError> {
        if output.status.success() {
            let str_stdout = String::from_utf8(output.stdout)?;
            let str_stderr = String::from_utf8(output.stderr)?;
            Ok(BenchmarkRecord::Data(
                (str_stdout + &str_stderr).trim_end().to_string(),
            ))
        } else {
            Err(CommandMeasurementError::ExecutionFailed(output))
        }
    }
}

impl MeasuringEquipment for CommandExecutor {
    type MeasurementError = CommandMeasurementError;

    fn execute(&self, point: BenchmarkPoint) -> Result<BenchmarkRecord, CommandMeasurementError> {
        Self::handle_output(
            self.0
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
    ) -> Result<BenchmarkRecord, CommandMeasurementError> {
        self.0
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
        let executor = CommandExecutor::new_time(cmd!("git", "fail"));
        let error = executor.execute(0).unwrap_err();
        assert!(matches!(error, CommandMeasurementError::ExecutionFailed(_)));
    }

    #[test]
    fn test_execution_timeout() {
        let executor = CommandExecutor::new_time(cmd!("sleep"));
        let output = executor
            .execute_with_timeout(2, std::time::Duration::from_secs(1))
            .unwrap();
        assert!(output.is_timeout());
    }
    #[test]
    fn test_execution_in_time() {
        let executor = CommandExecutor::new_time(cmd!("sleep"));
        let output = executor
            .execute_with_timeout(1, std::time::Duration::from_secs(2))
            .unwrap();
        assert!(!output.is_timeout());
    }
}

#[derive(Debug)]
pub(crate) struct FlameExecutor {
    flame_path: PathBuf,
    run_command: String,
}

impl FlameExecutor {
    fn build_pipe(&self, point: BenchmarkPoint) -> Pipeline {
        Exec::from(&cmd!(
            "perf",
            "record",
            "-F",
            "99",
            "-a",
            "-g",
            "-o",
            "-",
            "--",
            self.run_command.clone(),
            point.to_string()
        )) | Exec::from(&cmd!("perf", "script", "-i", "-"))
            | Exec::cmd(self.flame_path.join("stackcollapse-perf.pl"))
    }
}

#[justerror::Error]
pub(crate) enum FlameMeasuringError {
    FailedBuildingTheCommand(#[from] PopenError),
    #[error(fmt = debug)]
    FailedRunningThePipe(CaptureData),
    WrongOutputFormat(#[from] FromUtf8Error),
}

impl MeasuringEquipment for FlameExecutor {
    type MeasurementError = FlameMeasuringError;
    fn execute(&self, point: BenchmarkPoint) -> Result<BenchmarkRecord, Self::MeasurementError> {
        let captured = self.build_pipe(point).capture()?;
        if !captured.success() {
            return Err(FlameMeasuringError::FailedRunningThePipe(captured));
        }
        Ok(BenchmarkRecord::Data(String::from_utf8(captured.stdout)?))
    }

    fn execute_with_timeout(
        &self,
        point: BenchmarkPoint,
        timeout: Duration,
    ) -> Result<BenchmarkRecord, Self::MeasurementError> {
        todo!()
    }
}
