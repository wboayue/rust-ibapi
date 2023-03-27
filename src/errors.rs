#[derive(Debug)]
pub enum Error {
    // Errors from external libraries...
    // Io(io::Error),
    // Git(git2::Error),
    // Errors raised by us...
    Regular(ErrorKind),
    Simple(String)
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ErrorKind {
    NotAuthorized
}

impl ErrorKind {
    fn as_str(&self) -> &str {
        match *self {
            // ErrorKind::NotFound => "not found",
            ErrorKind::NotAuthorized => "not authorized"
        }
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            // MyErrorType::Git(ref err) => err.description(),
            Error::Regular(ref err) => err.as_str(),
            Error::Simple(ref err) => err,
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            // MyErrorType::Io(ref err) => err.fmt(f),
            // MyErrorType::Git(ref err) => err.fmt(f),
            Error::Regular(ref err) => write!(f, "A regular error occurred {:?}", err),
            Error::Simple(ref err) => write!(f, "A custom error occurred {:?}", err),
        }
    }
}

impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Error {
        Error::Simple(err.to_string())
    }
}

// impl From<git2::Error> for MyErrorType {
//     fn from(err: git2::Error) -> MyErrorType {
//         MyErrorType::Git(err)
//     }
// }
