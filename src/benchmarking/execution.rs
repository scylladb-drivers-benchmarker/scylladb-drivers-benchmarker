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

    CompilationStarting(#[from] std::io::Error),
    #[error(fmt=debug)]
    CompilationRunning {
        output: std::process::Output,
    },
}

pub fn build_source(build_command: &str) -> Result<BuiltSource, CompileError> {
    let command = Command::from_str(build_command)?;
    let mut command = command.process();

    let output = command.output()?;
    if output.status.success() {
        Ok(BuiltSource { _private: () })
    } else {
        Err(CompileError::CompilationRunning { output })
    }
}

#[justerror::Error(desc = "measuring failed")]
pub enum MeasurementError {
    #[error(fmt = debug)]
    ExecutionFailed(Output),
    // no documentation for how and when this error is thrown in Command.output
    RustFailed(#[from] io::Error),
    WrongOutputFormat(#[from] std::string::FromUtf8Error),
}

#[derive(Debug)]
pub struct Executor {
    command: Command,
}

impl Executor {
    pub fn new(_: BuiltSource, run_command: &str) -> Result<Executor, CommandParsingError> {
        let command = Command::from_str(run_command)?;
        Ok(Executor { command })
    }

    pub fn with_measure(self, measurement_method: &str) -> Result<Executor, CommandParsingError> {
        let measurement_command = Command::from_str(measurement_method)?;
        Ok(Executor {
            command: measurement_command.with_arg(self.command.to_string()),
        })
    }

    fn handle_output(output: Output) -> Result<BenchmarkRecord, MeasurementError> {
        if output.status.success() {
            let str_stdout = String::from_utf8(output.stdout)?;
            let str_stderr = String::from_utf8(output.stderr)?;
            Ok(BenchmarkRecord::Data(str_stdout + &str_stderr))
        } else {
            Err(MeasurementError::ExecutionFailed(output))
        }
    }

    pub fn execute_with_timeout(
        &self,
        param: BenchmarkPoint,
        timeout: std::time::Duration,
    ) -> Result<BenchmarkRecord, MeasurementError> {
        let Some(output) = self
            .command
            .clone()
            .with_arg(param.to_string())
            .output_with_timeout(timeout)?
        else {
            return Ok(BenchmarkRecord::Timeout);
        };
        Self::handle_output(output)
    }

    pub fn execute(&self, param: BenchmarkPoint) -> Result<BenchmarkRecord, MeasurementError> {
        let output: Output = self.command.clone().with_arg(param.to_string()).output()?;
        Self::handle_output(output)
    }
}

#[cfg(test)]
mod test {
    use super::{BuiltSource, Executor, MeasurementError};

    #[test]
    fn test_execution_error() {
        let source = BuiltSource { _private: () };
        let executor = Executor::new(source, "git fail").unwrap();
        let error = executor.execute(0).unwrap_err();
        assert!(matches!(error, MeasurementError::ExecutionFailed(_)));
    }

    #[test]
    fn test_execution_timeout() {
        let source = BuiltSource { _private: () };
        let executor = Executor::new(source, "sleep").unwrap();
        let output = executor
            .execute_with_timeout(2, std::time::Duration::from_secs(1))
            .unwrap();
        assert!(output.is_timeout());
    }
    #[test]
    fn test_execution_in_time() {
        let source = BuiltSource { _private: () };
        let executor = Executor::new(source, "sleep").unwrap();
        let output = executor
            .execute_with_timeout(1, std::time::Duration::from_secs(2))
            .unwrap();
        assert!(!output.is_timeout());
    }
}
