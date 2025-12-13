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

        if !output.status.success() {
            Err(MeasurementError::ExecutionFailed(output))
        } else {
            let str_stdout = String::from_utf8(output.stdout)?;
            let str_stderr = String::from_utf8(output.stderr)?;
            Ok(BenchmarkRecord::Data(str_stdout + &str_stderr))
        }
    }

    pub fn execute(&self, param: BenchmarkPoint) -> Result<BenchmarkRecord, MeasurementError> {
        let output: Output = self.command.clone().with_arg(param.to_string()).output()?;
        if !output.status.success() {
            Err(MeasurementError::ExecutionFailed(output))
        } else {
            let str_stdout = String::from_utf8(output.stdout)?;
            let str_stderr = String::from_utf8(output.stderr)?;
            Ok(BenchmarkRecord::Data(str_stdout + &str_stderr))
        }
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
