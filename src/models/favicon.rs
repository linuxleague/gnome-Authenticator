use super::CLIENT;
use image::io::Reader as ImageReader;
use once_cell::sync::Lazy;
use quick_xml::events::{attributes::Attribute, BytesStart, Event};
use std::io::Cursor;
use url::Url;

pub static FAVICONS_PATH: Lazy<std::path::PathBuf> = Lazy::new(|| {
    gtk::glib::get_user_cache_dir()
        .join("authenticator")
        .join("favicons")
});

const SUPPORTED_RELS: [&[u8]; 7] = [
    b"icon",
    b"fluid-icon",
    b"shortcut icon",
    b"apple-touch-icon",
    b"apple-touch-icon-precomposed",
    b"fluid-icon",
    b"alternate icon",
];

#[derive(Debug)]
pub enum FaviconError {
    Surf(surf::Error),
    Url(url::ParseError),
    NoResults,
}

impl From<surf::Error> for FaviconError {
    fn from(e: surf::Error) -> Self {
        Self::Surf(e)
    }
}

impl From<url::ParseError> for FaviconError {
    fn from(e: url::ParseError) -> Self {
        Self::Url(e)
    }
}

impl std::error::Error for FaviconError {}

impl std::fmt::Display for FaviconError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FaviconError::NoResults => write!(f, "FaviconError: No results were found"),
            e => write!(f, "FaviconError: {}", e),
        }
    }
}

#[derive(Debug)]
pub struct Favicon(Vec<Url>);

impl Favicon {
    pub async fn find_best(&self) -> Option<&Url> {
        let mut largest_size = 0;
        let mut best = None;
        for url in self.0.iter() {
            if let Some(size) = self.get_size(url).await {
                // Only store the width & assumes it has the same height here to simplify things
                if size.0 > largest_size {
                    largest_size = size.0;
                    best = Some(url);
                }
            }
        }
        best
    }

    pub async fn get_size(&self, url: &Url) -> Option<(u32, u32)> {
        let mut response = CLIENT.get(url).await.ok()?;

        let ext = std::path::Path::new(url.path())
            .extension()
            .map(|e| e.to_str().unwrap())?;

        if ext == "svg" {
            let body = response.body_string().await.ok()?;

            self.svg_dimensions(body)
        } else {
            let bytes = response.body_bytes().await.ok()?;

            let mut image = ImageReader::new(Cursor::new(bytes));

            let format = image::ImageFormat::from_extension(ext)?;
            image.set_format(format);
            image.into_dimensions().ok()
        }
    }

    // TODO: replace with librsvg maybe?
    fn svg_dimensions(&self, svg: String) -> Option<(u32, u32)> {
        let metadata = svg_metadata::Metadata::parse(svg).ok()?;

        let width = metadata.width()? as u32;
        let height = metadata.height()? as u32;
        Some((width, height))
    }
}
#[derive(Debug)]
pub struct FaviconScrapper;

impl FaviconScrapper {
    pub async fn from_url(url: Url) -> Result<Favicon, FaviconError> {
        let mut res = CLIENT.get(&url).header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 11_1) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/14.0.2 Safari/605.1.15").await?;
        let body = res.body_string().await?;
        let mut reader = quick_xml::Reader::from_str(&body);
        reader.check_end_names(false);
        reader.trim_markup_names_in_closing_tags(true);

        let icons = Self::from_reader(&mut reader, &url);
        if icons.is_empty() {
            return Err(FaviconError::NoResults);
        }
        Ok(Favicon(icons))
    }

    fn from_reader(reader: &mut quick_xml::Reader<&[u8]>, base_url: &Url) -> Vec<Url> {
        let mut buf = Vec::new();
        let mut urls = Vec::new();
        loop {
            match reader.read_event(&mut buf) {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    if let b"link" = e.name() {
                        if let Some(url) = Self::from_link(e, base_url) {
                            urls.push(url);
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => debug!("Error at position {}: {:?}", reader.buffer_position(), e),
                _ => (),
            }
        }
        buf.clear();
        urls
    }

    fn from_link(e: &BytesStart, base_url: &Url) -> Option<Url> {
        let mut url = None;

        let mut has_proper_rel = false;
        for attr in e.html_attributes() {
            match attr {
                Ok(Attribute {
                    key: b"href",
                    value,
                }) => {
                    let mut href = String::from_utf8(value.into_owned()).unwrap();
                    if href.starts_with("//") {
                        href = format!("https:{}", href);
                    }
                    url = match Url::parse(&href) {
                        Ok(url) => Some(url),
                        Err(url::ParseError::RelativeUrlWithoutBase) => base_url.join(&href).ok(),
                        Err(_) => None,
                    };
                }
                Ok(Attribute { key: b"rel", value }) => {
                    if SUPPORTED_RELS.contains(&value.into_owned().as_slice()) {
                        has_proper_rel = true;
                    }
                }
                _ => (),
            }
            if has_proper_rel && url.is_some() {
                break;
            }
        }
        if has_proper_rel {
            return url;
        }
        None
    }
}
