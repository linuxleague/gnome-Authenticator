use std::{fmt, path::Path};

use url::Url;

/// Supported image formats.
#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
pub enum Format {
    #[default]
    Png,
    Svg,
    Ico,
}

impl Format {
    /// Create a [`Format`] from a URL's path otherwise default to
    /// [`Format::Png`].
    ///
    /// ```
    /// use favicon_scrapper::Format;
    /// use url::Url;
    ///
    /// let url = Url::parse("http://127.0.0.1:8000/favicon.ico").unwrap();
    /// assert!(Format::from_url(&url).is_ico());
    ///
    /// let url = Url::parse("http://127.0.0.1:8000/favicon.png").unwrap();
    /// assert!(Format::from_url(&url).is_png());
    ///
    /// let url = Url::parse("http://127.0.0.1:8000/favicon.svg").unwrap();
    /// assert!(Format::from_url(&url).is_svg());
    /// ```
    pub fn from_url(url: &Url) -> Self {
        let ext = Path::new(url.path())
            .extension()
            .map(|e| e.to_str().unwrap_or_default());
        match ext {
            Some("png") => Self::Png,
            Some("ico") => Self::Ico,
            Some("svg") => Self::Svg,
            _ => Self::default(),
        }
    }

    /// Create a [`Format`] from a mimetype otherwise default to
    /// [`Format::Png`].
    ///
    /// ```
    /// use favicon_scrapper::Format;
    ///
    /// assert!(Format::from_mimetype("image/svg+xml").is_svg());
    /// assert!(Format::from_mimetype("image/png").is_png());
    /// assert!(Format::from_mimetype("image/x-icon").is_ico());
    pub fn from_mimetype(mimetype: &str) -> Self {
        match mimetype {
            "image/x-icon" => Self::Ico,
            "image/png" => Self::Png,
            "image/svg+xml" => Self::Svg,
            _ => Self::default(),
        }
    }

    pub fn is_svg(self) -> bool {
        matches!(self, Self::Svg)
    }

    pub fn is_png(self) -> bool {
        matches!(self, Self::Png)
    }

    pub fn is_ico(self) -> bool {
        matches!(self, Self::Ico)
    }
}

impl fmt::Display for Format {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Png => f.write_str("png"),
            Self::Ico => f.write_str("ico"),
            Self::Svg => f.write_str("svg"),
        }
    }
}
