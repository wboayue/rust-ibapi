use std::{num::ParseIntError, string::FromUtf8Error};

#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    // Errors from external libraries
    Io(std::io::Error),
    ParseInt(ParseIntError),
    FromUtf8(FromUtf8Error),
    ParseTime(time::error::Parse),
    Poison(String),

    // Errors from by IBAPI library
    NotImplemented,
    Parse(usize, String, String),
    ServerVersion(i32, i32, String),
    Simple(String),
    ConnectionFailed,
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Io(ref err) => err.fmt(f),
            Error::ParseInt(ref err) => err.fmt(f),
            Error::FromUtf8(ref err) => err.fmt(f),
            Error::ParseTime(ref err) => err.fmt(f),
            Error::Poison(ref err) => write!(f, "{}", err),

            Error::NotImplemented => write!(f, "not implemented"),
            Error::Parse(i, value, message) => write!(f, "parse error: {i} - {value} - {message}"),
            Error::ServerVersion(wanted, have, message) => write!(f, "server version {wanted} required, got {have}: {message}"),
            Error::ConnectionFailed => write!(f, "ConnectionFailed"),

            Error::Simple(ref err) => write!(f, "error occurred: {err}"),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::Io(err)
    }
}

impl From<ParseIntError> for Error {
    fn from(err: ParseIntError) -> Error {
        Error::ParseInt(err)
    }
}

impl From<FromUtf8Error> for Error {
    fn from(err: FromUtf8Error) -> Error {
        Error::FromUtf8(err)
    }
}

impl From<time::error::Parse> for Error {
    fn from(err: time::error::Parse) -> Error {
        Error::ParseTime(err)
    }
}

impl<T> From<std::sync::PoisonError<T>> for Error {
    fn from(err: std::sync::PoisonError<T>) -> Error {
        Error::Poison(format!("Mutex poison error: {}", err))
    }
}
