//! This module supports the execution and measuring of the benchmarked code
//! from arbitrary commands. It performs little to no validation of those
//! commands eg. whether the run command they actually interacts with the
//! output of the build command.

use std::str::FromStr;

use crate::command::{Command, CommandParsingError};
use crate::database::utilities::BenchmarkRecord;
use crate::measurement::{MeasurementError, MeasurementMethod, MeasuringEquipment};
use crate::utilities::BenchmarkPoint;

/// This is a token proving that the code being executed was compiled earlier.
/// Getting this from outside of this module happens only by invoking `build_source`.
#[must_use]
pub struct BuiltSource {
    _private: (),
}

impl BuiltSource {
    fn new_unchecked() -> Self {
        BuiltSource { _private: () }
    }
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

/// Builds the source code, using the provided command.
/// This is the only way to receive `BuiltSource` from outside.
pub fn build_source(build_command: &str) -> Result<BuiltSource, CompileError> {
    let command = Command::from_str(build_command)?;
    let mut command = command.process();

    let output = command.output()?;
    if output.status.success() {
        Ok(BuiltSource::new_unchecked())
    } else {
        Err(CompileError::CompilationRunning { output })
    }
}

#[derive(Debug)]
pub struct Executor {
    measure: MeasurementMethod,
    run_command: Command,
}

impl Executor {
    pub fn new(
        _: BuiltSource,
        run_command_str: &str,
        measure: MeasurementMethod,
    ) -> Result<Executor, CommandParsingError> {
        let run_command = Command::from_str(run_command_str)?;
        Ok(Executor {
            measure,
            run_command,
        })
    }

    pub fn execute_with_timeout(
        &self,
        param: BenchmarkPoint,
        timeout: std::time::Duration,
    ) -> Result<BenchmarkRecord, MeasurementError> {
        let full_run_command = self.run_command.clone().with_arg(param.to_string());
        self.measure.execute_with_timeout(full_run_command, timeout)
    }

    pub fn execute(&self, param: BenchmarkPoint) -> Result<BenchmarkRecord, MeasurementError> {
        let full_run_command = self.run_command.clone().with_arg(param.to_string());
        self.measure.execute(full_run_command)
    }
}

#[cfg(test)]
mod test {
    use crate::{
        benchmarking::execution::BuiltSource,
        measurement::{self, MeasurementError},
    };

    use super::Executor;

    #[test]
    fn test_execution_error() {
        let executor = Executor::new(
            BuiltSource::new_unchecked(),
            "git fail",
            measurement::Time {}.into(),
        )
        .unwrap();
        let error = executor.execute(0).unwrap_err();
        assert!(matches!(error, MeasurementError::ExecutionFailed(_)));
    }

    #[test]
    fn test_execution_timeout() {
        let executor = Executor::new(
            BuiltSource::new_unchecked(),
            "sleep",
            measurement::Time {}.into(),
        )
        .unwrap();
        let output = executor
            .execute_with_timeout(2, std::time::Duration::from_secs(1))
            .unwrap();
        assert!(output.is_timeout());
    }
    #[test]
    fn test_execution_in_time() {
        let executor = Executor::new(
            BuiltSource::new_unchecked(),
            "sleep",
            measurement::Time {}.into(),
        )
        .unwrap();
        let output = executor
            .execute_with_timeout(1, std::time::Duration::from_secs(2))
            .unwrap();
        assert!(!output.is_timeout());
    }
}
