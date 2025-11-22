use std::error::Error;
use std::fmt;
use std::path::PathBuf;

#[derive(Debug)]
pub struct ConfigurationNotFound {
    pub path: PathBuf,
    pub name: String,
}

impl fmt::Display for ConfigurationNotFound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let path = self.path.display();
        let name = &self.name;
        write!(f, "Unable to find configuration: {name} in {path}")
    }
}

impl Error for ConfigurationNotFound {}