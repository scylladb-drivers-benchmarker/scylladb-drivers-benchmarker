use std::{error::Error, path::PathBuf, process::Output};

use crate::utilities::BenchmarkRecord;
use crate::{command::Command, database};

pub struct SourceCode {
    pub path: Option<PathBuf>,
}

pub struct CompiledSource {}

pub fn compile(
    compilation_command: &str,
    source_code: SourceCode,
) -> Result<CompiledSource, Box<dyn Error>> {
    let mut command = Command::new(compilation_command)?.process();

    if let Some(path) = source_code.path {
        command.current_dir(path);
    }

    command.output()?;
    Ok(CompiledSource {})
}

pub type ErrBenchmarkResult = Result<BenchmarkRecord, Box<dyn Error>>;

pub struct Executor {
    command: Command,
}

impl Executor {
    pub fn new(_: CompiledSource, execution_command: &str) -> Result<Executor, Box<dyn Error>> {
        let command = Command::new(execution_command)?;
        Ok(Executor { command })
    }

    pub fn measure(&mut self, instrumentation: &str) -> Result<(), Box<dyn Error>> {
        let instrumentation_command = Command::new(instrumentation)?;
        self.command = instrumentation_command.with_arg(self.command.to_string());
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
