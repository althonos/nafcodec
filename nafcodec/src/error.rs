//! Common error type for this crate.

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Nom(nom::error::Error<Vec<u8>>),
    Utf8(std::str::Utf8Error),
    InvalidSequence,
    InvalidLength,
    MissingField(&'static str),
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Error::Io(error)
    }
}

impl From<nom::error::Error<Vec<u8>>> for Error {
    fn from(error: nom::error::Error<Vec<u8>>) -> Self {
        Error::Nom(error)
    }
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(error: std::string::FromUtf8Error) -> Self {
        Self::from(error.utf8_error())
    }
}

impl From<std::str::Utf8Error> for Error {
    fn from(error: std::str::Utf8Error) -> Self {
        Error::Utf8(error)
    }
}

impl<'i> From<nom::error::Error<&'i [u8]>> for Error {
    fn from(error: nom::error::Error<&'i [u8]>) -> Self {
        Error::Nom(nom::error::Error::new(error.input.to_owned(), error.code))
    }
}

impl<E> From<nom::Err<E>> for Error
where
    E: Into<Error>,
{
    fn from(error: nom::Err<E>) -> Self {
        match error {
            nom::Err::Error(e) | nom::Err::Failure(e) => e.into(),
            nom::Err::Incomplete(_) => todo!(),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => e.fmt(f),
            Error::Nom(e) => nom::Err::Error(e).fmt(f),
            Error::Utf8(e) => e.fmt(f),
            Error::InvalidLength => f.write_str("inconsistent sequence length"),
            Error::InvalidSequence => f.write_str("invalid character in sequence"),
            Error::MissingField(field) => write!(f, "missing record field: {:?}", field),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            Error::Utf8(e) => Some(e),
            Error::Nom(_) => None,
            Error::InvalidLength => None,
            Error::InvalidSequence => None,
            Error::MissingField(_) => None,
        }
    }
}
