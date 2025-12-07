use std::io;
use std::process::Output;
use std::str::FromStr;

use crate::command::{Command, CommandParsingError};
use crate::utilities::{BenchmarkPoint, BenchmarkRecord};

pub struct BuiltSource {
    _private: (),
}

#[justerror::Error(desc = "compilation failed")]
pub enum CompileError {
    CommandParsing(#[from] CommandParsingError),

    CompilationRunning(#[from] std::io::Error),
}

pub fn build_source(build_command: impl AsRef<str>) -> Result<BuiltSource, CompileError> {
    let command = Command::from_str(build_command.as_ref())?;
    let mut command = command.process();

    command.output()?;
    Ok(BuiltSource { _private: () })
}

#[justerror::Error(desc = "measuring failed")]
pub enum MeasurementError {
    #[error(fmt = debug)]
    ExecutionFailed(Output),
    // no documentation for how and when this error is thrown in Command.output
    RustFailed(#[from] io::Error),
    WrongOutputFormat(#[from] std::string::FromUtf8Error),
}

pub struct Executor {
    command: Command,
}

impl Executor {
    pub fn new(
        _: BuiltSource,
        run_command: impl AsRef<str>,
    ) -> Result<Executor, CommandParsingError> {
        let command = Command::from_str(run_command.as_ref())?;
        Ok(Executor { command })
    }

    pub fn with_measure(
        self,
        measurement_method: impl AsRef<str>,
    ) -> Result<Executor, CommandParsingError> {
        let measurement_command = Command::from_str(measurement_method.as_ref())?;
        Ok(Executor {
            command: measurement_command.with_arg(self.command.to_string()),
        })
    }

    pub fn execute(&self, param: BenchmarkPoint) -> Result<BenchmarkRecord, MeasurementError> {
        let output: Output = self
            .command
            .clone()
            .with_arg(param.to_string())
            .process()
            .output()?;
        if !output.status.success() {
            Err(MeasurementError::ExecutionFailed(output))
        } else {
            let measurement_result = String::from_utf8(output.stdout)?;
            Ok(BenchmarkRecord::new(Some(measurement_result)))
        }
    }

    #[allow(dead_code)]
    pub fn command(&self) -> &Command {
        &self.command
    }
}
