use std::error::Error;
use std::path::Path;
use std::process::{self, Output};
use std::time::Duration;
use std::{fs, io};

use log::trace;
use tempfile::NamedTempFile;

use crate::benchmarking::executor::MeasuringEquipment;
use crate::command::OutputWithTimeout;
use crate::database::utilities::BenchmarkRecord;
use crate::utilities::BenchmarkPoint;
use crate::{cmd, command};

#[derive(Debug)]
pub(crate) struct OutputExecutor {
    make_command: fn(&Path, command::Command) -> command::Command,
    run_command: command::Command,
}

#[justerror::Error(desc = "measuring failed")]
pub(crate) enum OutputExecutorError {
    TempFile(#[source] io::Error),
    FileReadingFailed(#[source] io::Error),
    CommandBuildingFailed(#[source] io::Error),
    #[error(fmt = debug)]
    ExecutionFailed(Output),
}

impl OutputExecutor {
    pub(crate) fn new_time(run_command: command::Command) -> Self {
        OutputExecutor {
            make_command: |path: &Path, command| {
                cmd!("time", "-o", path.display().to_string(), "-f", "%e").with_cmd_arg(command)
            },
            run_command,
        }
    }
    pub(crate) fn new_perf_stat(run_command: command::Command) -> Self {
        OutputExecutor {
            make_command: |path: &Path, command| {
                cmd!("perf", "stat", "--json", "-o", path.display().to_string())
                    .with_cmd_arg(command)
            },
            run_command,
        }
    }

    fn handle_output(
        output: Output,
        file: &fs::File,
    ) -> Result<BenchmarkRecord, OutputExecutorError> {
        if output.status.success() {
            trace!("Reading from output...");
            let mut measured = io::read_to_string(file)
                .map_err(OutputExecutorError::FileReadingFailed)?;
            measured.truncate(measured.trim_end().len());
            Ok(BenchmarkRecord::Data(measured))
        } else {
            Err(OutputExecutorError::ExecutionFailed(output))
        }
    }

    fn general_execute(
        &self,
        point: BenchmarkPoint,
        execute: impl FnOnce(&mut process::Command) -> Result<Option<Output>, io::Error>,
    ) -> Result<BenchmarkRecord, OutputExecutorError> {
        trace!("Creating output file...");
        let output_file = NamedTempFile::new().map_err(OutputExecutorError::TempFile)?;
        
        let mut command = (self.make_command)(
            output_file.path(),
            self.run_command.clone().with_arg(point.to_string()),
        )
        .process();

        trace!("Executing the run command...");
        let output =
            execute(&mut command).map_err(OutputExecutorError::CommandBuildingFailed)?;

        match output {
            None => {
                trace!("Timed out");
                Ok(BenchmarkRecord::Timeout)
            },
            Some(output) => {
                trace!("Run command finished");
                Self::handle_output(output, output_file.as_file())
            },
        }
    }
}

impl MeasuringEquipment for OutputExecutor {
    fn execute(&self, point: BenchmarkPoint) -> Result<BenchmarkRecord, Box<dyn Error + 'static>> {
        self.general_execute(point, |command| command.output().map(Some))
            .map_err(|e| Box::new(e) as Box<dyn Error + 'static>)
    }

    fn execute_with_timeout(
        &self,
        point: BenchmarkPoint,
        timeout: Duration,
    ) -> Result<BenchmarkRecord, Box<dyn Error + 'static>> {
        self.general_execute(point, |command| command.output_with_timeout(timeout))
            .map_err(|e| Box::new(e) as Box<dyn Error + 'static>)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_execution_error() {
        let executor = OutputExecutor::new_time(cmd!("git", "fail"));
        let error: OutputExecutorError = *executor.execute(0).unwrap_err().downcast().unwrap();
        assert!(matches!(error, OutputExecutorError::ExecutionFailed(_)));
    }

    #[test]
    fn test_execution_timeout() {
        let executor = OutputExecutor::new_time(cmd!("sleep"));
        let output = executor
            .execute_with_timeout(2, std::time::Duration::from_secs(1))
            .unwrap();
        assert!(output.is_timeout());
    }
    #[test]
    fn test_execution_in_time() {
        let executor = OutputExecutor::new_time(cmd!("sleep"));
        let output = executor
            .execute_with_timeout(1, std::time::Duration::from_secs(2))
            .unwrap();
        assert!(!output.is_timeout());
    }
}
