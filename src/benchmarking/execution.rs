use std::path::PathBuf;
use std::process::Output;
use std::str::FromStr;

use crate::command::{Command, CommandParsingError};
use crate::utilities::{BenchmarkPoint, BenchmarkRecord, BenchmarkResult};

pub struct SourceCode {
    pub path: Option<PathBuf>,
}

pub struct BuiltSource {}

#[justerror::Error(desc = "compilation failed")]
pub enum CompileError {
    CommandParsingError(
        #[from]
        #[source]
        CommandParsingError,
    ),

    CompilationRunningError(
        #[from]
        #[source]
        std::io::Error,
    ),
}

pub fn build_source(
    build_command: &str,
    source_code: SourceCode,
) -> Result<BuiltSource, CompileError> {
    let command = Command::from_str(build_command)?;
    let mut command = command.process();

    if let Some(path) = source_code.path {
        command.current_dir(path);
    }

    command.output()?;
    Ok(BuiltSource {})
}

pub struct Executor {
    command: Command,
}

impl Executor {
    pub fn new(_: BuiltSource, execution_command: &str) -> Result<Executor, CommandParsingError> {
        let command = Command::from_str(execution_command)?;
        Ok(Executor { command })
    }

    pub fn with_measure(self, measurement_method: Command) -> Executor {
        Executor {
            command: measurement_method.with_arg(self.command.to_string()),
        }
    }

    pub fn execute(&self, param: BenchmarkPoint) -> std::io::Result<Output> {
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

    pub fn run(&self, param: BenchmarkPoint) -> BenchmarkResult {
        let output = self.execute(param)?;
        let output = String::from_utf8(output.stdout)?;
        Ok(BenchmarkRecord::new(Some(output)))
    }
}

#[cfg(test)]
mod test {
    use shell_words::ParseError;

    use crate::benchmarking::execution::CompileError;
    use crate::command::CommandParsingError;

    #[test]
    fn test() {
        println!(
            "{}",
            CompileError::CommandParsingError(CommandParsingError::FailedShlexing(ParseError))
        );
        panic!();
    }
}
