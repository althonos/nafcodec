//! Common error type for this crate.

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Nom(nom::error::Error<Vec<u8>>),
    Utf8(std::str::Utf8Error),
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
