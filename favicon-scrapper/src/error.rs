#[derive(Debug)]
pub enum Error {
    Reqwest(reqwest::Error),
    Url(url::ParseError),
    Io(std::io::Error),
    Image(image::ImageError),
    NoResults,
    Utf8(std::str::Utf8Error),
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Self::Reqwest(e)
    }
}

impl From<url::ParseError> for Error {
    fn from(e: url::ParseError) -> Self {
        Self::Url(e)
    }
}

impl From<image::ImageError> for Error {
    fn from(e: image::ImageError) -> Self {
        Self::Image(e)
    }
}

impl From<std::str::Utf8Error> for Error {
    fn from(e: std::str::Utf8Error) -> Self {
        Self::Utf8(e)
    }
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoResults => write!(f, "Error: No results were found"),
            Self::Reqwest(e) => write!(f, "Reqwest Error {e}"),
            Self::Url(e) => write!(f, "Url Parse Error{e}"),
            Self::Io(e) => write!(f, "IO Error {e}"),
            Self::Image(e) => write!(f, "Image Error {e}"),
            Self::Utf8(e) => write!(f, "String conversion error {e}"),
        }
    }
}
