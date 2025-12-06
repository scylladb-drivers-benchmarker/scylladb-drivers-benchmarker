use std::io;
use std::path::PathBuf;
use std::process::Output;
use std::str::FromStr;

use crate::command::{Command, CommandParsingError};
use crate::utilities::{BenchmarkPoint, BenchmarkRecord};

pub struct SourceCode {
    pub path: Option<PathBuf>,
}

pub struct BuiltSource {}

#[justerror::Error(desc = "compilation failed")]
pub enum CompileError {
    CommandParsingError(
        #[from]
        #[source]
        CommandParsingError,
    ),

    CompilationRunningError(
        #[from]
        #[source]
        std::io::Error,
    ),
}

pub fn build_source(
    build_command: impl AsRef<str>,
    source_code: SourceCode,
) -> Result<BuiltSource, CompileError> {
    let command = Command::from_str(build_command.as_ref())?;
    let mut command = command.process();

    if let Some(path) = source_code.path {
        command.current_dir(path);
    }

    command.output()?;
    Ok(BuiltSource {})
}

#[justerror::Error(desc = "measuring failed")]
pub enum MeasurementError {
    #[error(fmt = debug)]
    ExecutionFailed(Output),
    // no documentation for how and when this error is thrown in Command.output
    RustFailed(
        #[from]
        #[source]
        io::Error,
    ),
    WrongOutputFormat(
        #[from]
        #[source]
        std::string::FromUtf8Error,
    ),
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

    pub fn command(&self) -> &Command {
        &self.command
    }
}
