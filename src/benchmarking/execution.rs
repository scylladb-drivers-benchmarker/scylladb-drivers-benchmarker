use std::{
    error::Error,
    path::{Path, PathBuf},
    process::{self, Output},
};

use crate::benchmarking::command::Command;

pub struct SourceCode {
    pub path: Option<PathBuf>,
}

pub struct CompiledSource {}

pub fn compile(
    compilation_command: String,
    source_code: SourceCode,
) -> Result<CompiledSource, Box<dyn Error>> {
    let mut command = Command::new(compilation_command)?.process();

    if let Some(path) = source_code.path {
        command.current_dir(path);
    }

    command.output()?;
    Ok(CompiledSource {})
}

pub struct Executor {
    command: Command,
}

impl Executor {
    pub fn new(_: CompiledSource, execution_command: String) -> Result<Executor, Box<dyn Error>> {
        let command = Command::new(execution_command)?;
        Ok(Executor { command })
    }

    pub fn measure(&mut self, instrumentation: String) -> Result<(), Box<dyn Error>> {
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
}
