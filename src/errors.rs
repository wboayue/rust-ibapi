use std::{num::ParseIntError, string::FromUtf8Error, sync::Arc};

use crate::messages::ResponseMessage;

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Error {
    // Errors from external libraries
    Io(Arc<std::io::Error>),
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
    ConnectionReset,
    Cancelled,
    Shutdown,
    StreamEnd,
    UnexpectedResponse(ResponseMessage),
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
            Error::ConnectionReset => write!(f, "ConnectionReset"),
            Error::Cancelled => write!(f, "Cancelled"),
            Error::Shutdown => write!(f, "Shutdown"),
            Error::StreamEnd => write!(f, "StreamEnd"),
            Error::UnexpectedResponse(message) => write!(f, "UnexpectedResponse: {:?}", message),

            Error::Simple(ref err) => write!(f, "error occurred: {err}"),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::Io(Arc::new(err))
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error as StdError;
    use std::io;
    use std::sync::{Mutex, PoisonError};
    use time::macros::format_description;
    use time::Time;

    #[test]
    fn test_error_debug() {
        let error = Error::Simple("test error".to_string());
        assert_eq!(format!("{:?}", error), "Simple(\"test error\")");
    }

    #[test]
    fn test_error_display() {
        let cases = vec![
            (
                Error::Io(Arc::new(io::Error::new(io::ErrorKind::NotFound, "file not found"))),
                "file not found",
            ),
            (Error::ParseInt("123x".parse::<i32>().unwrap_err()), "invalid digit found in string"),
            (
                Error::FromUtf8(String::from_utf8(vec![0, 159, 146, 150]).unwrap_err()),
                "invalid utf-8 sequence of 1 bytes from index 1",
            ),
            (
                Error::ParseTime(Time::parse("2021-13-01", format_description!("[year]-[month]-[day]")).unwrap_err()),
                "the 'month' component could not be parsed",
            ),
            (Error::Poison("test poison".to_string()), "test poison"),
            (Error::NotImplemented, "not implemented"),
            (
                Error::Parse(1, "value".to_string(), "message".to_string()),
                "parse error: 1 - value - message",
            ),
            (
                Error::ServerVersion(2, 1, "old version".to_string()),
                "server version 2 required, got 1: old version",
            ),
            (Error::ConnectionFailed, "ConnectionFailed"),
            (Error::Cancelled, "Cancelled"),
            (Error::Simple("simple error".to_string()), "error occurred: simple error"),
        ];

        for (error, expected) in cases {
            assert_eq!(error.to_string(), expected);
        }
    }

    #[test]
    fn test_error_is_error() {
        let error = Error::Simple("test error".to_string());
        assert!(error.source().is_none());
    }

    #[test]
    fn test_from_io_error() {
        let io_error = io::Error::new(io::ErrorKind::Other, "io error");
        let error: Error = io_error.into();
        assert!(matches!(error, Error::Io(_)));
    }

    #[test]
    fn test_from_parse_int_error() {
        let parse_error = "abc".parse::<i32>().unwrap_err();
        let error: Error = parse_error.into();
        assert!(matches!(error, Error::ParseInt(_)));
    }

    #[test]
    fn test_from_utf8_error() {
        let utf8_error = String::from_utf8(vec![0, 159, 146, 150]).unwrap_err();
        let error: Error = utf8_error.into();
        assert!(matches!(error, Error::FromUtf8(_)));
    }

    #[test]
    fn test_from_parse_time_error() {
        let time_error = Time::parse("2021-13-01", format_description!("[year]-[month]-[day]")).unwrap_err();
        let error: Error = time_error.into();
        assert!(matches!(error, Error::ParseTime(_)));
    }

    #[test]
    fn test_from_poison_error() {
        let mutex = Mutex::new(());
        let poison_error = PoisonError::new(mutex);
        let error: Error = poison_error.into();
        assert!(matches!(error, Error::Poison(_)));
    }

    #[test]
    fn test_non_exhaustive() {
        fn assert_non_exhaustive<T: StdError>() {}
        assert_non_exhaustive::<Error>();
    }
}
