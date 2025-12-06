use core::fmt;
use std::str::FromStr;

/// Struct used for parsing command from configs
#[derive(Debug, Clone)]
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
    #[error(desc = "shexing failed")]
    FailedShlexing( #[from] shell_words::ParseError),
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
    pub fn new_program(program: String) -> Self {
        Command {
            program,
            arguments: Vec::new(),
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

    pub fn add_argument(&mut self, argument: String) {
        self.arguments.push(argument);
    }

    pub fn with_arg(mut self, argument: String) -> Self {
        self.add_argument(argument);
        self
    }

    pub fn program(&self) -> &str {
        self.program.as_str()
    }

    pub fn args(&self) -> &Vec<String> {
        &self.arguments
    }

    pub fn process(self) -> std::process::Command {
        let mut command = std::process::Command::new(self.program());
        command.args(self.arguments);
        command
    }
}

/// Utility macro to create an explicit command.
#[macro_export]
macro_rules! cmd {
    ( $program:expr, $( $arg:expr ), *) => {
        Command::new_args(String::from($program), vec!($(String::from($arg), )*).iter())
    };
}

#[cfg(test)]
mod test {
    use std::{error::Error, ffi::OsStr};

    use super::*;

    #[test]
    fn command_from_str() {
        let mut command = Command::from_str("git commit -m \"This is a commit message\"").unwrap();
        command.add_argument("--author".to_owned());
        command.add_argument("This is a commit author".to_owned());
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
