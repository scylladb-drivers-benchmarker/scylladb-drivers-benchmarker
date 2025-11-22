use shlex::Shlex;

/// Struct used for parsing command from configs
pub struct Command {
    program: String,
    arguments: Vec<String>,
}

impl ToString for Command {
    fn to_string(&self) -> String {
        self.arguments
            .iter()
            .fold(self.program.clone(), |string, arg| string + arg)
    }
}

impl Command {
    /// Create a new command from a shell formatted string.
    /// ```
    /// let command = Command::new("git commit -m \"This is a commit message\"".to_owned());
    /// command.add_argument("--author");
    /// command.add_argument("This is a commit author")
    /// let process_cmd = command.process();
    /// assert!(process_cmd.get_program() == "git");
    /// let args: Vec<&OsStr> = process_cmd.get_args().collect();
    /// assert_eq!(args[0], "commit");
    /// assert_eq!(args[1], "-m");
    /// assert_eq!(args[2], "This is a commit message");
    /// assert_eq!(args[3], "--author");
    /// assert_eq!(args[4], "This is a commit author");
    /// ```
    pub fn new(command: String) -> Result<Self, String> {
        let mut it = Shlex::new(command.as_ref());
        let program: String = it.next().ok_or("Program name not given")?;
        if it.had_error {
            return Err("Failed parsing program name.".to_string());
        }

        let mut arguments: Vec<String> = Vec::new();
        while let Some(next_value) = it.next() {
            arguments.push(next_value);
        }

        if it.had_error {
            let erroneous = arguments.last().expect("Shlex should first mark had_error before stopping");
            Err(String::from("Wrong argument: ") + erroneous)
        } else {
            Ok(Command { program, arguments })
        }
    }

    pub fn new_args<Args: Iterator<Item = impl ToString>>(program: String, arguments: Args) -> Self {
        Command {
            program,
            arguments: arguments.map(|item| item.to_string()).collect(),
        }
    }

    fn add_argument(&mut self, argument: String) {
        self.arguments.push(argument);
    }

    fn with_arg(mut self, argument: String) -> Self {
        self.add_argument(argument);
        self
    }

    fn get_program(&self) -> &str {
        self.program.as_str()
    }

    fn get_args(&self) -> impl Iterator<Item = impl AsRef<str>> {
        self.arguments.iter()
    }

    fn process(self) -> std::process::Command {
        let mut command = std::process::Command::new(self.get_program());
        command.args(self.arguments);
        command
    }
}
