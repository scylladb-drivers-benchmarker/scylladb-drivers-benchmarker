use std::fs::File;
use std::io::{self};
use std::path::{Path, PathBuf};
use std::time::Duration;

use subprocess::{Exec, ExitStatus, Pipeline, Popen, PopenError, Redirection};
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

#[justerror::Error]
pub(crate) enum FlameMeasuringError {
    FailedBuildingThePipe(#[fmt(debug)] Pipeline, #[source] PopenError),
    #[error(desc = "command still running after the pipe finish")]
    SubExecStillRunning {
        command: command::Command,
    },
    #[error(desc = "failed running a subcommand")]
    SubExecFailed {
        #[fmt(debug)]
        exit_status: subprocess::ExitStatus,
        stderr: String,
        command: command::Command,
    },
    PipeFailure(#[from] PopenError),
    OutputFileError(#[from] io::Error),
}

impl FlameMeasuringError {
    fn new_failure(exit_status: ExitStatus, mut popen: Popen, command: command::Command) -> Self {
        let stderr = popen.communicate_bytes(None).unwrap().1.unwrap();
        FlameMeasuringError::SubExecFailed {
            exit_status,
            stderr: String::from_utf8_lossy(&stderr).to_string(),
            command,
        }
    }
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

    fn general_execute(
        &self,
        point: BenchmarkPoint,
        last_wait: impl FnOnce(&mut Popen) -> Result<Option<ExitStatus>, FlameMeasuringError>,
    ) -> Result<BenchmarkRecord, FlameMeasuringError> {
        let commands = [
            cmd!(
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
            cmd!("perf", "script", "-i", "-"),
            cmd!(
                self.flame_repo
                    .join("stackcollapse-perf.pl")
                    .to_string_lossy()
                    .to_string()
            ),
        ];

        let (filepath, file) = self.next_file()?;

        let make_pipeline = || {
            Pipeline::from_exec_iter(
                commands
                    .iter()
                    .map(|command| Exec::from(command).stderr(Redirection::Pipe)),
            )
        };

        let pipeline = make_pipeline().stdout(file);

        let mut popens = match pipeline.popen() {
            Ok(popens) => popens,
            Err(err) => {
                return Err(FlameMeasuringError::FailedBuildingThePipe(
                    make_pipeline(),
                    err,
                ));
            }
        };

        let mut last_popen = popens.pop().expect("pipe should be not empty");

        let Some(exit_status) = last_wait(&mut last_popen)? else {
            return Ok(BenchmarkRecord::Timeout);
        };

        for (index, mut popen) in popens.into_iter().enumerate() {
            let Some(exit_status) = popen.poll() else {
                return Err(FlameMeasuringError::SubExecStillRunning {
                    command: commands.into_iter().nth(index).unwrap(),
                });
            };
            if !exit_status.success() {
                return Err(FlameMeasuringError::new_failure(
                    exit_status,
                    popen,
                    commands.into_iter().nth(index).unwrap(),
                ));
            }
        }

        if !exit_status.success() {
            let [.., last] = commands;
            return Err(FlameMeasuringError::new_failure(
                exit_status,
                last_popen,
                last,
            ));
        }
        Ok(BenchmarkRecord::FilePath(filepath))
    }
}

impl MeasuringEquipment for FlameExecutor {
    type MeasurementError = FlameMeasuringError;
    fn execute(&self, point: BenchmarkPoint) -> Result<BenchmarkRecord, Self::MeasurementError> {
        self.general_execute(point, |popen| Ok(Some(popen.wait()?)))
    }

    fn execute_with_timeout(
        &self,
        point: BenchmarkPoint,
        timeout: Duration,
    ) -> Result<BenchmarkRecord, Self::MeasurementError> {
        self.general_execute(point, |popen| Ok(popen.wait_timeout(timeout)?))
    }
}
