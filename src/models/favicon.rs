use image::io::Reader as ImageReader;
use quick_xml::events::{attributes::Attribute, BytesStart, Event};
use std::io::Cursor;
use url::Url;
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
pub struct Favicon(Vec<Url>, surf::Client);

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
        let mut response = self.1.get(url).await.ok()?;

        let ext = std::path::Path::new(url.path())
            .extension()
            .map(|e| e.to_str().unwrap())?;
        // Assumes the svg is the best size we can find
        if ext == "svg" {
            return Some((1024, 1024));
        }

        let format = match ext {
            "png" => image::ImageFormat::Png,
            "ico" => image::ImageFormat::Ico,
            _ => unreachable!(),
        };

        let bytes = response.body_bytes().await.ok()?;
        let mut image = ImageReader::new(Cursor::new(bytes));
        image.set_format(format);
        image.into_dimensions().ok()
    }
}
#[derive(Debug)]
pub struct FaviconScrapper(surf::Client);

impl FaviconScrapper {
    pub fn new() -> Self {
        let client = surf::client().with(surf::middleware::Redirect::default());
        Self(client)
    }

    pub async fn from_url(&self, url: Url) -> Result<Favicon, FaviconError> {
        let mut res = self.0.get(&url).header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 11_1) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/14.0.2 Safari/605.1.15").await?;
        let body = res.body_string().await?;
        let mut reader = quick_xml::Reader::from_str(&body);
        reader.check_end_names(false);
        reader.trim_markup_names_in_closing_tags(true);

        let icons = self.from_reader(&mut reader, &url);
        if icons.is_empty() {
            return Err(FaviconError::NoResults);
        }
        Ok(Favicon(icons, self.0.clone()))
    }

    fn from_reader(&self, reader: &mut quick_xml::Reader<&[u8]>, base_url: &Url) -> Vec<Url> {
        let mut buf = Vec::new();
        let mut urls = Vec::new();
        loop {
            match reader.read_event(&mut buf) {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    if let b"link" = e.name() {
                        if let Some(url) = self.from_link(e, base_url) {
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

    fn from_link(&self, e: &BytesStart, base_url: &Url) -> Option<Url> {
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
