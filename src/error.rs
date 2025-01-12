use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid regex: {0}")]
    BadRegex(#[from] regex::Error),
    #[error("Failed to install colors: \"{0:?}\"")]
    FailedToInstallColors(#[from] color_eyre::Report),
    #[error("IO action failed: \"{0:?}\"")]
    IO(#[from] std::io::Error),
    #[error("Unknown command: \"{0}\"")]
    UnknownCommand(String),
    #[error("No arguments were passed to the last command")]
    NoArgumentsGivenToCommand,
}

pub type Result<T> = std::result::Result<T, Error>;
