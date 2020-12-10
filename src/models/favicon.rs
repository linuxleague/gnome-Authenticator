use quick_xml::events::{attributes::Attribute, BytesStart, Event};
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
    GLib(glib::Error),
    NoResults,
}

impl From<surf::Error> for FaviconError {
    fn from(e: surf::Error) -> Self {
        Self::Surf(e)
    }
}

impl From<glib::Error> for FaviconError {
    fn from(e: glib::Error) -> Self {
        Self::GLib(e)
    }
}

impl From<url::ParseError> for FaviconError {
    fn from(e: url::ParseError) -> Self {
        Self::Url(e)
    }
}

pub struct Favicon {
    icons: Vec<Url>,
}

impl Favicon {}
#[derive(Debug)]
pub struct FaviconScrapper;

impl FaviconScrapper {
    pub async fn from_url(url: Url) -> Result<Vec<Url>, FaviconError> {
        let mut res = surf::get(&url).await?;
        let body = res.body_string().await?;
        let mut reader = quick_xml::Reader::from_str(&body);

        let icons = Self::from_reader(&mut reader, &url);

        Ok(icons)
    }

    fn from_reader(reader: &mut quick_xml::Reader<&[u8]>, base_url: &Url) -> Vec<Url> {
        let mut buf = Vec::new();
        let mut urls = Vec::new();
        loop {
            match reader.read_event(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    if let b"link" = e.name() {
                        if let Some(url) = Self::from_link(e, base_url) {
                            urls.push(url);
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => warn!("Error at position {}: {:?}", reader.buffer_position(), e),
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
                    let href = String::from_utf8(value.into_owned()).unwrap();
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
