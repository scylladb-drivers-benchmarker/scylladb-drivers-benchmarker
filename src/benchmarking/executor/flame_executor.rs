use core::time;
use std::fs::File;
use std::io;
use std::iter::once;
use std::path::{Path, PathBuf};
use std::string::FromUtf8Error;
use std::time::Duration;

use subprocess::{Exec, Pipeline, Popen, PopenError, Redirection};
use uuid::Uuid;

use crate::benchmarking::executor::MeasuringEquipment;
use crate::database::utilities::BenchmarkRecord;
use crate::flame_graph::FlameFrequency;
use crate::utilities::BenchmarkPoint;
use crate::{cmd, command};

#[derive(Debug)]
pub(crate) struct FlameExecutor {
    flame_repo: PathBuf,
    store_dir: PathBuf,
    frequency: String,
    run_command: command::Command,
}

impl FlameExecutor {
    fn exec_list(&self, point: BenchmarkPoint) -> impl Iterator<Item = Exec> {
        let execs = once(Exec::from(
            &cmd!(
                "perf",
                "record",
                "-F",
                &self.frequency,
                "-a",
                "-g",
                "-o",
                "-",
                "--"
            )
            .with_cmd_arg(self.run_command.clone())
            .with_arg(point.to_string()),
        ))
        .chain(once(Exec::from(&cmd!("perf", "script", "-i", "-"))))
        .chain(once(Exec::cmd(
            self.flame_repo.join("stackcollapse-perf.pl"),
        )));
        return execs.map(|exec| exec.stderr(Redirection::Pipe));
    }

    fn build_pipe(&self, point: BenchmarkPoint) -> Pipeline {
        Pipeline::from_exec_iter(self.exec_list(point))
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
    SubExecStillRunning(),
    SubExecFailed {
        #[fmt(debug)]
        exit_status: subprocess::ExitStatus,
        stderr: String,
    },
    OutputFileError(#[from] io::Error),
}

impl FlameExecutor {
    pub(crate) fn new(
        flame_repo: PathBuf,
        store_dir: PathBuf,
        frequency: FlameFrequency,
        run_command: command::Command,
    ) -> Self {
        FlameExecutor {
            flame_repo,
            store_dir,
            frequency: frequency.to_string(),
            run_command,
        }
    }

    fn next_file(&self) -> Result<(PathBuf, File), FlameMeasuringError> {
        let filename = self.store_dir.join(Path::new(&Uuid::new_v4().to_string()));
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

    fn check_others(&self, sub_execs: Vec<Popen>) -> Result<(), FlameMeasuringError> {
        for mut sub_exec in sub_execs {
            let exit_status = match sub_exec.poll() {
                None => return Err(FlameMeasuringError::SubExecStillRunning()),
                Some(exit_status) => exit_status,
            };
            if !exit_status.success() {
                let stderr = sub_exec.communicate_bytes(None).unwrap().1.unwrap();
                return Err(FlameMeasuringError::SubExecFailed {
                    exit_status,
                    stderr: String::from_utf8_lossy(&stderr).to_string(),
                });
            }
        }
        return Ok(());
    }
}

impl MeasuringEquipment for FlameExecutor {
    type MeasurementError = FlameMeasuringError;
    fn execute(&self, point: BenchmarkPoint) -> Result<BenchmarkRecord, Self::MeasurementError> {
        let (filepath, file) = self.next_file()?;
        let mut sub_execs = self
            .build_pipe(point)
            .stdout(file)
            .popen()
            .map_err(self.wrap_building(point))?;

        let mut last_popen = sub_execs.pop().expect("pipe should be not empty");
        last_popen.wait().map_err(|_err| -> FlameMeasuringError {
            todo!();
        })?;
        self.check_others(sub_execs)?;
        Ok(BenchmarkRecord::FilePath(filepath))
    }

    fn execute_with_timeout(
        &self,
        point: BenchmarkPoint,
        timeout: Duration,
    ) -> Result<BenchmarkRecord, Self::MeasurementError> {
        let (filepath, file) = self.next_file()?;
        let mut sub_execs = self
            .build_pipe(point)
            .stdout(file)
            .popen()
            .map_err(self.wrap_building(point))?;

        let mut last_popen = sub_execs.pop().expect("pipe should be not empty");
        last_popen
            .wait_timeout(timeout)
            .map_err(|_err| -> FlameMeasuringError {
                todo!();
            })?;
        self.check_others(sub_execs)?;
        Ok(BenchmarkRecord::FilePath(filepath))
    }
}
