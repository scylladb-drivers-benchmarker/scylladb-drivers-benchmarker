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
    if !output.status.success() {
        Err(CompileError::CompilationRunning { output })
    } else {
        Ok(BuiltSource { _private: () })
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
    pub fn new(
        _: BuiltSource,
        run_command: &str,
    ) -> Result<Executor, CommandParsingError> {
        let command = Command::from_str(run_command)?;
        Ok(Executor { command })
    }

    pub fn with_measure(
        self,
        measurement_method: &str,
    ) -> Result<Executor, CommandParsingError> {
        let measurement_command = Command::from_str(measurement_method)?;
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
}
