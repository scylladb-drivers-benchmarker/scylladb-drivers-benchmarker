use core::fmt;
use std::ffi::OsStr;
use std::fmt::Display;
use std::io::{BufReader, Read};
use std::process;
use std::str::FromStr;
use wait_timeout::ChildExt;

/// Struct used for parsing command from configs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    program: String,
    arguments: Vec<String>,
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.program)?;

        for arg in &self.arguments {
            write!(f, " {}", arg)?;
        }

        Ok(())
    }
}

#[justerror::Error]
#[derive(PartialEq, Eq)]
pub enum CommandParsingError {
    #[error(desc = "program not given")]
    ProgramNotGiven,
    #[error(desc = "shlexing failed")]
    FailedShlexing(#[from] shell_words::ParseError),
}

impl FromStr for Command {
    type Err = CommandParsingError;

    fn from_str(command: &str) -> Result<Self, Self::Err> {
        let mut words = shell_words::split(command)?;
        if words.is_empty() {
            Err(CommandParsingError::ProgramNotGiven)
        } else {
            Ok(Command {
                program: words.remove(0),
                arguments: words,
            })
        }
    }
}

impl Command {
    #[cfg(test)]
    pub fn new_program(program: String) -> Self {
        Command {
            program,
            arguments: Vec::new(),
        }
    }

    pub fn from_command_lossy(command: &process::Command) -> Self {
        Command {
            program: command.get_program().to_string_lossy().to_string(),
            arguments: command
                .get_args()
                .map(OsStr::to_string_lossy)
                .map(String::from)
                .collect(),
        }
    }

    pub fn new_args<Args: Iterator<Item = impl ToString>>(
        program: String,
        arguments: Args,
    ) -> Self {
        Command {
            program,
            arguments: arguments.map(|item| item.to_string()).collect(),
        }
    }

    pub fn with_arg(mut self, argument: String) -> Self {
        self.add_arg(argument);
        self
    }

    pub fn add_arg(&mut self, argument: String) {
        self.arguments.push(argument);
    }

    pub fn with_args(mut self, arguments: impl Iterator<Item = String>) -> Self {
        arguments.for_each(|arg| self.add_arg(arg));
        self
    }

    pub fn with_cmd_arg(self, argument: Command) -> Self {
        self.with_arg(argument.program)
            .with_args(argument.arguments.into_iter())
    }

    pub fn program(&self) -> &str {
        self.program.as_str()
    }

    #[cfg(test)]
    pub fn args(&self) -> &Vec<String> {
        &self.arguments
    }

    pub fn process(self) -> std::process::Command {
        let mut command = std::process::Command::new(self.program());
        command.args(self.arguments);
        command
    }
}

pub trait OutputWithTimeout {
    fn output_with_timeout(
        &mut self,
        timeout: std::time::Duration,
    ) -> Result<Option<std::process::Output>, std::io::Error>;
}

impl OutputWithTimeout for process::Command {
    fn output_with_timeout(
        &mut self,
        timeout: std::time::Duration,
    ) -> Result<Option<std::process::Output>, std::io::Error> {
        let mut child: std::process::Child = self
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        let Some(status) = child.wait_timeout(timeout)? else {
            return Ok(None);
        };

        let buffer = BufReader::new(child.stdout.unwrap());
        let stdout = buffer.bytes().collect::<Result<Vec<u8>, _>>()?;

        let buffer = BufReader::new(child.stderr.unwrap());
        let stderr = buffer.bytes().collect::<Result<Vec<u8>, _>>()?;

        Ok(Some(std::process::Output {
            status,
            stdout,
            stderr,
        }))
    }
}

/// Utility macro to create an explicit command.
#[macro_export]
macro_rules! cmd {
    ( $program:expr, $( $arg:expr ), *) => {
        $crate::command::Command::new_args(String::from($program), vec!($(String::from($arg), )*).iter())
    };
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrintableOutput(process::Output);

impl Into<PrintableOutput> for process::Output {
    fn into(self) -> PrintableOutput {
        PrintableOutput(self)
    }
}

impl Display for PrintableOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "status: {}", self.0.status)?;
        write!(
            f,
            "stdout: \n\"\n{}\"\n",
            String::from_utf8_lossy(&self.0.stdout)
        )?;
        write!(
            f,
            "stderr: \n\"\n{}\"\n",
            String::from_utf8_lossy(&self.0.stderr)
        )
    }
}

#[cfg(test)]
mod test {
    use std::ffi::OsStr;

    use super::*;

    #[test]
    fn command_from_str() {
        let command = Command::from_str("git commit -m \"This is a commit message\"")
            .unwrap()
            .with_arg("--author".to_owned())
            .with_arg("This is a commit author".to_owned());
        let process_cmd = command.process();
        assert!(process_cmd.get_program() == "git");
        let args: Vec<&OsStr> = process_cmd.get_args().collect();
        assert_eq!(args[0], "commit");
        assert_eq!(args[1], "-m");
        assert_eq!(args[2], "This is a commit message");
        assert_eq!(args[3], "--author");
        assert_eq!(args[4], "This is a commit author");
    }

    #[test]
    fn from_str_error() {
        assert_eq!(
            Command::from_str("").unwrap_err(),
            CommandParsingError::ProgramNotGiven
        );

        assert_eq!(
            Command::from_str("\"unfinished quoting").unwrap_err(),
            CommandParsingError::FailedShlexing(shell_words::ParseError)
        );

        assert_eq!(
            Command::from_str("git -m \"unfinished quoting").unwrap_err(),
            CommandParsingError::FailedShlexing(shell_words::ParseError)
        );
    }

    #[test]
    fn program() {
        let command = Command::new_program("git".to_owned());
        assert!(command.program() == "git");
        assert!(*command.args() == Vec::<String>::new());
    }

    #[test]
    fn with_args() {
        let command = Command::new_args(
            "git".to_owned(),
            vec!["--author", "Author", "-m", "\"This is a commit message\""].into_iter(),
        );
        assert_eq!(command.program(), "git".to_owned());

        let args: &Vec<String> = command.args();
        assert_eq!(args[0], "--author");
        assert_eq!(args[1], "Author");
        assert_eq!(args[2], "-m");
        assert_eq!(args[3], "\"This is a commit message\"");
    }

    #[test]
    fn cmd_macro() {
        let command = cmd!("git", "status", "-s");
        assert_eq!(command.program(), "git");
        assert_eq!(*command.args(), vec!("status", "-s"))
    }
}
