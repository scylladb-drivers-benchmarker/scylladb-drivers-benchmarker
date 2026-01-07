use std::io;
use std::process::Output;
use std::time::Duration;

use enum_dispatch::enum_dispatch;

use crate::{
    cmd,
    command::{self, OutputWithTimeout},
    database::utilities::BenchmarkRecord,
    measurement::MeasurementMethod,
    utilities::BenchmarkPoint,
};

#[justerror::Error(desc = "measuring failed")]
pub enum MeasurementError {
    #[error(fmt = debug)]
    ExecutionFailed(Output),
    // no documentation for how and when this error is thrown in Command.output
    RustFailed(#[from] io::Error),
    WrongOutputFormat(#[from] std::string::FromUtf8Error),
}

#[enum_dispatch(Measurer)]
pub trait MeasuringEquipment {
    fn execute(&self, point: BenchmarkPoint) -> Result<BenchmarkRecord, MeasurementError>;
    fn execute_with_timeout(
        &self,
        point: BenchmarkPoint,
        duration: Duration,
    ) -> Result<BenchmarkRecord, MeasurementError>;
}

#[enum_dispatch]
#[derive(Debug)]
pub(crate) enum Measurer {
    CommandMeasurer,
}

impl Measurer {
    pub fn new(measurement_method: MeasurementMethod, run_command: command::Command) -> Self {
        match measurement_method {
            MeasurementMethod::Time => {
                CommandMeasurer::new(cmd!("time", "-f", "%e"), run_command).into()
            }
            MeasurementMethod::Perf => {
                CommandMeasurer::new(cmd!("perf", "stat", "--json"), run_command).into()
            }
            MeasurementMethod::Flamegraph => todo!(),
            MeasurementMethod::Command(_) => todo!(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct CommandMeasurer {
    command: command::Command,
}

impl CommandMeasurer {
    fn new(command: command::Command, run_command: command::Command) -> Self {
        CommandMeasurer {
            command: command.with_cmd_arg(run_command),
        }
    }
}

impl CommandMeasurer {
    fn handle_output(output: Output) -> Result<BenchmarkRecord, MeasurementError> {
        if output.status.success() {
            let str_stdout = String::from_utf8(output.stdout)?;
            let str_stderr = String::from_utf8(output.stderr)?;
            Ok(BenchmarkRecord::Data(str_stdout + &str_stderr))
        } else {
            Err(MeasurementError::ExecutionFailed(output))
        }
    }
}

impl MeasuringEquipment for CommandMeasurer {
    fn execute(&self, point: BenchmarkPoint) -> Result<BenchmarkRecord, MeasurementError> {
        Self::handle_output(
            self.command
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
    ) -> Result<BenchmarkRecord, MeasurementError> {
        self.command
            .clone()
            .with_arg(point.to_string())
            .process()
            .output_with_timeout(timeout)?
            .map(Self::handle_output)
            .unwrap_or(Ok(BenchmarkRecord::Timeout))
    }
}
