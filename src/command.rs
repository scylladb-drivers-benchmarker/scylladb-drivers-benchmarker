use core::fmt;
use std::str::FromStr;

use shlex::Shlex;

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

impl FromStr for Command {
    type Err = String;

    fn from_str(command: &str) -> Result<Self, Self::Err> {
        let mut it = Shlex::new(command);
        let program: String = it.next().ok_or("Program name not given")?;
        if it.had_error {
            return Err("Failed parsing program name.".to_string());
        }

        let mut arguments: Vec<String> = Vec::new();
        for next_value in it.by_ref() {
            arguments.push(next_value);
        }

        if it.had_error {
            let erroneous = arguments
                .last()
                .expect("Shlex should first mark had_error before stopping");
            Err(String::from("Wrong argument: ") + erroneous)
        } else {
            Ok(Command { program, arguments })
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
    fn command_from_str() -> Result<(), Box<dyn Error + 'static>> {
        let mut command = Command::from_str("git commit -m \"This is a commit message\"")?;
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
        Ok(())
    }

    #[test]
    fn cmd_macro() {
        let command = cmd!("git", "status", "-s");
        assert_eq!(command.program(), "git");
        assert_eq!(*command.args(), vec!("status", "-s"))
    }
}
