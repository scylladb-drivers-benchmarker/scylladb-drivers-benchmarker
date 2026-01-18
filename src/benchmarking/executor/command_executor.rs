use std::process::Output;
use std::time::Duration;

use crate::benchmarking::executor::{CommandMeasurementError, MeasuringEquipment};
use crate::command::OutputWithTimeout;
use crate::database::utilities::BenchmarkRecord;
use crate::utilities::BenchmarkPoint;
use crate::{cmd, command};

#[derive(Debug)]
pub(crate) struct CommandExecutor(command::Command);

impl CommandExecutor {
    pub(crate) fn new(command: command::Command, run_command: command::Command) -> Self {
        CommandExecutor(command.with_cmd_arg(run_command))
    }

    pub(crate) fn new_time(run_command: command::Command) -> Self {
        CommandExecutor::new(cmd!("time", "-f", "%e"), run_command)
    }

    pub(crate) fn new_perf(run_command: command::Command) -> Self {
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
            .map_or(Ok(BenchmarkRecord::Timeout), Self::handle_output)
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
