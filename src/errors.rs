use std::{num::ParseIntError, string::FromUtf8Error};

#[derive(Debug)]
pub enum Error {
    // Errors from external libraries...
    Io(std::io::Error),
    ParseInt(ParseIntError),
    FromUtf8(FromUtf8Error),

    // Errors raised by us...
    Regular(ErrorKind),
    Simple(String),
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ErrorKind {
    NotImplemented,
    Parse(usize, String, String),
    ServerVersion(i32, i32, String),
}

impl ErrorKind {
    fn as_str(&self) -> String {
        match self {
            // ErrorKind::NotFound => "not found",
            ErrorKind::NotImplemented => "not implemented".into(),
            ErrorKind::Parse(i, value, message) => format!("parse error: {} - {} - {}", i, value, message),
            ErrorKind::ServerVersion(wanted, have, message) => format!("server version {} required, got {}: {}", wanted, have, &message),
        }
    }
}

// impl std::fmt::Display for Error {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             Error::Io(ref err) => &err.to_string(),
//             Error::ParseInt(ref err) => err.to_string().into(),
//             Error::FromUtf8(ref err) => err.to_string().into(),

//             Error::Regular(ref err) => err.as_str(),
//             Error::Simple(ref err) => err,
//         }
//     }
// }

impl std::error::Error for Error {
    // fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    //     None
    // }

    // fn type_id(&self, _: private::Internal) -> std::any::TypeId
    // where
    //     Self: 'static,
    // {
    //     std::any::TypeId::of::<Self>()
    // }

    // fn description(&self) -> &str {
    //     "description() is deprecated; use Display"
    // }

    // fn cause(&self) -> Option<&dyn std::error::Error> {
    //     self.source()
    // }

    // fn provide<'a>(&'a self, demand: &mut std::any::Demand<'a>) {}

    // fn description(&self) -> &str {
    //     match *self {
    //         Error::Io(ref err) => &err.to_string(),
    //         Error::ParseInt(ref err) => &err.to_string(),
    //         Error::FromUtf8(ref err) => &err.to_string(),

    //         Error::Regular(ref err) => &err.as_str(),
    //         Error::Simple(ref err) => err,
    //     }
    // }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Error::Io(ref err) => err.fmt(f),
            Error::ParseInt(ref err) => err.fmt(f),
            Error::FromUtf8(ref err) => err.fmt(f),

            Error::Regular(ref err) => write!(f, "A regular error occurred {:?}", err),
            Error::Simple(ref err) => write!(f, "A custom error occurred {:?}", err),
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
