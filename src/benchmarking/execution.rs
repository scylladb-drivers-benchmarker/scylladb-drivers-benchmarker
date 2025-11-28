use std::str::FromStr;
use std::{error::Error, path::PathBuf, process::Output};

use crate::utilities::BenchmarkRecord;
use crate::{command::Command, database};

pub struct SourceCode {
    pub path: Option<PathBuf>,
}

pub struct BuiltSource {}

pub fn build_source(
    build_command: &str,
    source_code: SourceCode,
) -> Result<BuiltSource, Box<dyn Error>> {
    let mut command = Command::from_str(build_command)?.process();

    if let Some(path) = source_code.path {
        command.current_dir(path);
    }

    command.output()?;
    Ok(BuiltSource {})
}

pub type ErrBenchmarkResult = Result<BenchmarkRecord, Box<dyn Error>>;

pub struct Executor {
    command: Command,
}

impl Executor {
    pub fn new(_: BuiltSource, execution_command: &str) -> Result<Executor, Box<dyn Error>> {
        let command = Command::from_str(execution_command)?;
        Ok(Executor { command })
    }

    pub fn measure(&mut self, measurement_method: &str) -> Result<(), Box<dyn Error>> {
        let measurement_method_command = Command::from_str(measurement_method)?;
        self.command = measurement_method_command.with_arg(self.command.to_string());
        Ok(())
    }

    pub fn execute(&self, param: u32) -> Result<Output, Box<dyn Error>> {
        let output: Output = self
            .command
            .clone()
            .with_arg(param.to_string())
            .process()
            .output()?;
        Ok(output)
    }

    pub fn command(&self) -> &Command {
        &self.command
    }

    pub fn run(&self, param: u32) -> ErrBenchmarkResult {
        let output = self.execute(param)?;
        let output = String::from_utf8(output.stdout)?;
        Ok(BenchmarkRecord::new(Some(output)))
    }
}
