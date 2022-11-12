use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Error(String);

impl Error {
    pub fn new<S: AsRef<str>>(msg: S) -> Self {
        Self(String::from(msg.as_ref()))
    }
}

impl<T> From<T> for Error
where
    T: std::error::Error,
{
    fn from(e: T) -> Self {
        Self(format!("{}", e))
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

pub type Result<T> = core::result::Result<T, Error>;
