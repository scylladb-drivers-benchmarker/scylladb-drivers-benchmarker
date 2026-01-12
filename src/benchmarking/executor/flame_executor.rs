use std::io;
use std::path::PathBuf;
use std::string::FromUtf8Error;
use std::time::Duration;

use subprocess::{CaptureData, Exec, Pipeline, PopenError};

use crate::benchmarking::executor::MeasuringEquipment;
use crate::database::utilities::BenchmarkRecord;
use crate::utilities::BenchmarkPoint;
use crate::{cmd, command};

#[derive(Debug)]
pub(crate) struct FlameExecutor {
    flame_path: PathBuf,
    run_command: command::Command,
}

impl FlameExecutor {
    fn build_pipe(&self, point: BenchmarkPoint) -> Pipeline {
        Exec::from(
            &cmd!("perf", "record", "-F", "99", "-a", "-g", "-o", "-", "--")
                .with_cmd_arg(self.run_command.clone())
                .with_arg(point.to_string()),
        ) | Exec::from(&cmd!("perf", "script", "-i", "-"))
            | Exec::cmd(self.flame_path.join("stackcollapse-perf.pl"))
    }
}

#[justerror::Error]
pub(crate) enum FlameMeasuringError {
    #[error(fmt = debug)]
    FailedBuildingTheCommand {
        error: PopenError,
        pipe: Pipeline,
    },
    #[error(fmt = debug)]
    FailedRunningThePipe(CaptureData),
    #[error(fmt = debug)]
    FailedRunningThePipeInTimeout {
        stdout: String,
        stderr: String,
    },
    WrongOutputFormat(#[from] FromUtf8Error),
}

impl FlameExecutor {
    pub(crate) fn new(flame_path: PathBuf, run_command: command::Command) -> Self {
        FlameExecutor {
            flame_path,
            run_command,
        }
    }

    fn collect_output(pair: (Option<Vec<u8>>, Option<Vec<u8>>)) -> (Vec<u8>, Vec<u8>) {
        let (stdout, stderr) = pair;
        (
            stdout.expect("subscribed to stdout"),
            stderr.expect("subscribed to stderr"),
        )
    }
}

impl MeasuringEquipment for FlameExecutor {
    type MeasurementError = FlameMeasuringError;
    fn execute(&self, point: BenchmarkPoint) -> Result<BenchmarkRecord, Self::MeasurementError> {
        let captured = self.build_pipe(point).capture().map_err(|error| {
            FlameMeasuringError::FailedBuildingTheCommand {
                error,
                pipe: self.build_pipe(point),
            }
        })?;
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
        let mut communicator = self.build_pipe(point).communicate().map_err(|error| {
            FlameMeasuringError::FailedBuildingTheCommand {
                error,
                pipe: self.build_pipe(point),
            }
        })?;
        communicator = communicator.limit_time(timeout);
        let captured = match communicator.read() {
            Err(error) => {
                let (stdout, stderr) = Self::collect_output(error.capture);
                return match error.error.kind() {
                    io::ErrorKind::TimedOut => Ok(BenchmarkRecord::Timeout),
                    _ => Err(FlameMeasuringError::FailedRunningThePipeInTimeout {
                        stdout: String::from_utf8_lossy(&stdout).to_string(),
                        stderr: String::from_utf8_lossy(&stderr).to_string(),
                    }),
                };
            }
            Ok(val) => val,
        };

        Ok(BenchmarkRecord::Data(String::from_utf8(
            captured.0.unwrap(),
        )?))
    }
}
