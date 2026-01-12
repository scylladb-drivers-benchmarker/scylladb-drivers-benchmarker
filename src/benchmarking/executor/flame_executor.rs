use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use std::string::FromUtf8Error;
use std::time::Duration;

use subprocess::{CaptureData, CommunicateError, Exec, Pipeline, PopenError, Redirection};
use uuid::Uuid;

use crate::benchmarking::executor::MeasuringEquipment;
use crate::database::utilities::BenchmarkRecord;
use crate::utilities::BenchmarkPoint;
use crate::{cmd, command};

#[derive(Debug)]
pub(crate) struct FlameExecutor {
    flame_path: PathBuf,
    files_path: Option<PathBuf>,
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
    FailedBuildingThePipe(#[fmt(debug)] Pipeline, #[source] PopenError),
    #[error(desc = "failed running\n")]
    FailedRunningThePipe {
        #[fmt(debug)]
        exit_status: subprocess::ExitStatus,
        stdout: String,
    },
    #[error(desc = "capturing timed out")]
    FailedRunningThePipeInTimeout(),
    #[error(desc = "the output is not in utf8 format")]
    WrongOutputFormat(#[from] FromUtf8Error),
    OutputFileError(#[from] io::Error),
    NotADirectory(PathBuf),
    NoPathForOutput(),
}

impl FlameExecutor {
    pub(crate) fn new(
        flame_path: PathBuf,
        files_path: Option<PathBuf>,
        run_command: command::Command,
    ) -> Self {
        FlameExecutor {
            flame_path,
            files_path,
            run_command,
        }
    }

    fn files_path(&self) -> Result<&Path, FlameMeasuringError> {
        let Some(files_path) = &self.files_path else {
            return Err(FlameMeasuringError::NoPathForOutput());
        };
        if !files_path.is_dir() {
            return Err(FlameMeasuringError::NotADirectory(files_path.to_owned()));
        }
        return Ok(files_path);
    }

    fn next_file(&self) -> Result<(PathBuf, File), FlameMeasuringError> {
        let filename = self
            .files_path()?
            .join(Path::new(&Uuid::new_v4().to_string()));
        let file = File::options().create(true).write(true).open(&filename)?;
        Ok((filename, file))
    }

    fn wrap_building(
        &self,
        point: BenchmarkPoint,
    ) -> impl FnOnce(PopenError) -> FlameMeasuringError {
        move |source: PopenError| {
            FlameMeasuringError::FailedBuildingThePipe(self.build_pipe(point), source)
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
        let captured = self
            .build_pipe(point)
            .capture()
            .map_err(self.wrap_building(point))?;
        if !captured.success() {
            return Err(FlameMeasuringError::FailedRunningThePipe {
                exit_status: captured.exit_status,
                stdout: captured.stdout_str(),
            });
        }
        Ok(BenchmarkRecord::Data(String::from_utf8(captured.stdout)?))
    }

    fn execute_with_timeout(
        &self,
        point: BenchmarkPoint,
        timeout: Duration,
    ) -> Result<BenchmarkRecord, Self::MeasurementError> {
        let mut communicator = self
            .build_pipe(point)
            .communicate()
            .map_err(self.wrap_building(point))?;
        communicator = communicator.limit_time(timeout);
        let captured = match communicator.read() {
            Err(error) => {
                let (stdout, stderr) = Self::collect_output(error.capture);
                return match error.error.kind() {
                    io::ErrorKind::TimedOut => Ok(BenchmarkRecord::Timeout),
                    _ => Err(FlameMeasuringError::FailedRunningThePipeInTimeout()),
                };
            }
            Ok(val) => val,
        };

        Ok(BenchmarkRecord::Data(String::from_utf8(
            captured.0.unwrap(),
        )?))
    }
}
