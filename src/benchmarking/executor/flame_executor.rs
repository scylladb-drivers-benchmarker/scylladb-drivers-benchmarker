use std::error::Error;
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
        let stderr = popen
            .communicate_bytes(None)
            .ok()
            .map(|pipes| pipes.1.expect("stderr has to be piped"))
            .unwrap_or_default();
        FlameMeasuringError::SubExecFailed {
            exit_status,
            stderr: String::from_utf8_lossy(&stderr).into(),
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
        let file = fs_err::File::options()
            .create(true)
            .write(true)
            .open(&filename)?;
        Ok((filename, file.into_file()))
    }

    fn commands(&self, point: BenchmarkPoint) -> [command::Command; 3] {
        [
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
            ),
        ]
    }

    fn make_pipeline<'a>(commands: impl Iterator<Item = &'a command::Command>) -> Pipeline {
        Pipeline::from_exec_iter(
            commands.map(|command| Exec::from(command).stderr(Redirection::Pipe)),
        )
        .stdin(Redirection::None)
    }

    fn validate_pipeline_results(
        popens: Vec<Popen>,
        commands: impl Iterator<Item = command::Command>,
    ) -> Result<(), FlameMeasuringError> {
        for (mut popen, command) in popens.into_iter().zip(commands) {
            let Some(exit_status) = popen.poll() else {
                return Err(FlameMeasuringError::SubExecStillRunning { command });
            };
            if !exit_status.success() {
                return Err(FlameMeasuringError::new_failure(
                    exit_status,
                    popen,
                    command,
                ));
            }
        }
        Ok(())
    }

    fn general_execute(
        &self,
        point: BenchmarkPoint,
        last_wait: impl FnOnce(&mut Popen) -> Result<Option<ExitStatus>, FlameMeasuringError>,
    ) -> Result<BenchmarkRecord, FlameMeasuringError> {
        let commands = self.commands(point);

        let (filepath, file) = self.next_file()?;

        let pipeline = Self::make_pipeline(commands.iter()).stdout(file);

        let mut popens = match pipeline.popen() {
            Ok(popens) => popens,
            Err(err) => {
                return Err(FlameMeasuringError::FailedBuildingThePipe(
                    Self::make_pipeline(commands.iter()),
                    err,
                ));
            }
        };

        let mut last_popen = popens.pop().expect("pipe should be not empty");

        let Some(exit_status) = last_wait(&mut last_popen)? else {
            return Ok(BenchmarkRecord::Timeout);
        };

        if !exit_status.success() {
            let [.., last] = commands;
            return Err(FlameMeasuringError::new_failure(
                exit_status,
                last_popen,
                last,
            ));
        }

        Self::validate_pipeline_results(popens, commands.into_iter())?;

        Ok(BenchmarkRecord::FilePath(filepath))
    }
}

impl MeasuringEquipment for FlameExecutor {
    fn execute(&self, point: BenchmarkPoint) -> Result<BenchmarkRecord, Box<dyn Error + 'static>> {
        self.general_execute(point, |popen| Ok(Some(popen.wait()?)))
            .map_err(|err| Box::new(err) as Box<dyn Error + 'static>)
    }

    fn execute_with_timeout(
        &self,
        point: BenchmarkPoint,
        timeout: Duration,
    ) -> Result<BenchmarkRecord, Box<dyn Error + 'static>> {
        self.general_execute(point, |popen| Ok(popen.wait_timeout(timeout)?))
            .map_err(|err| Box::new(err) as Box<dyn Error + 'static>)
    }
}
