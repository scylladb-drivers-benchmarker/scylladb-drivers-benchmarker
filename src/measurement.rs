use std::fmt::{self, Debug, Display};
use std::io;
use std::process::Output;
use std::str::FromStr;
use std::time::Duration;

use enum_dispatch::enum_dispatch;

use crate::cmd;
use crate::command::{Command, CommandParsingError};
use crate::database::utilities::BenchmarkRecord;

#[justerror::Error(desc = "measuring failed")]
pub enum MeasurementError {
    #[error(fmt = debug)]
    ExecutionFailed(Output),
    // no documentation for how and when this error is thrown in Command.output
    RustFailed(#[from] io::Error),
    WrongOutputFormat(#[from] std::string::FromUtf8Error),
}

#[enum_dispatch(MeasurementMethod)]
pub trait MeasuringEquipment: Debug + Clone + Display + Eq {
    fn execute(&self, cmd: Command) -> Result<BenchmarkRecord, MeasurementError>;
    fn execute_with_timeout(
        &self,
        cmd: Command,
        duration: Duration,
    ) -> Result<BenchmarkRecord, MeasurementError>;
}

trait MeasuringCommand {
    fn to_command(&self) -> Command;
}

fn handle_output(output: Output) -> Result<BenchmarkRecord, MeasurementError> {
    if output.status.success() {
        let str_stdout = String::from_utf8(output.stdout)?;
        let str_stderr = String::from_utf8(output.stderr)?;
        Ok(BenchmarkRecord::Data(str_stdout + &str_stderr))
    } else {
        Err(MeasurementError::ExecutionFailed(output))
    }
}

impl<T: MeasuringCommand + Debug + Clone + Display + Eq> MeasuringEquipment for T {
    fn execute(&self, cmd: Command) -> Result<BenchmarkRecord, MeasurementError> {
        handle_output(self.to_command().with_cmd_arg(cmd).output()?)
    }

    fn execute_with_timeout(
        &self,
        cmd: Command,
        timeout: Duration,
    ) -> Result<BenchmarkRecord, MeasurementError> {
        self.to_command()
            .with_cmd_arg(cmd)
            .output_with_timeout(timeout)?
            .map(handle_output)
            .unwrap_or(Ok(BenchmarkRecord::Timeout))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Flamegraph {}

impl fmt::Display for Flamegraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "flamegraph")
    }
}

impl MeasuringEquipment for Flamegraph {
    fn execute(&self, _: Command) -> Result<BenchmarkRecord, MeasurementError> {
        todo!()
    }

    fn execute_with_timeout(
        &self,
        _: Command,
        _: Duration,
    ) -> Result<BenchmarkRecord, MeasurementError> {
        todo!()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Perf {}

impl fmt::Display for Perf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "perf")
    }
}

impl MeasuringCommand for Perf {
    fn to_command(&self) -> Command {
        cmd!("perf", "stat", "--json")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Time {}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "time")
    }
}

impl MeasuringCommand for Time {
    fn to_command(&self) -> Command {
        cmd!("time", "-f", "\"%e\"")
    }
}

impl MeasuringCommand for Command {
    fn to_command(&self) -> Command {
        self.clone()
    }
}

#[enum_dispatch]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeasurementMethod {
    Flamegraph,
    Perf,
    Time,
    Command,
}

impl Display for MeasurementMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MeasurementMethod::Flamegraph(flamegraph) => Display::fmt(flamegraph, f),
            MeasurementMethod::Perf(perf) => Display::fmt(perf, f),
            MeasurementMethod::Time(time) => Display::fmt(time, f),
            MeasurementMethod::Command(command) => Display::fmt(command, f),
        }
    }
}

#[justerror::Error]
pub enum MeasurementMethodParsingError {
    #[error(desc = "measuring method, not one of default, and not a command")]
    ParsingFailed(#[from] CommandParsingError),
}

impl FromStr for MeasurementMethod {
    type Err = MeasurementMethodParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "perf" => Ok(Perf {}.into()),
            "flamegraph" => Ok(Flamegraph {}.into()),
            "time" => Ok(Time {}.into()),
            value => Ok(MeasurementMethod::Command(Command::from_str(value)?)),
        }
    }
}
