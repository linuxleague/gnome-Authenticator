use log::debug;
use percent_encoding::percent_decode_str;
use quick_xml::events::{attributes::Attribute, BytesStart, Event};
use std::{fmt, path::PathBuf};
use url::Url;

use crate::{Error, Favicon, Format, Metadata, CLIENT};

const SUPPORTED_RELS: [&[u8]; 7] = [
    b"icon",
    b"fluid-icon",
    b"shortcut icon",
    b"apple-touch-icon",
    b"apple-touch-icon-precomposed",
    b"fluid-icon",
    b"alternate icon",
];

const SUPPORTED_META: [&[u8]; 1] = [b"msapplication-TileImage"];

pub struct Scrapper(Vec<Favicon>);

impl Scrapper {
    pub async fn from_url(base_url: Url) -> Result<Self, Error> {
        let res = CLIENT.get(base_url.as_str())
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 11_1) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/14.0.2 Safari/605.1.15")
            .send()
            .await?;
        let body = res.text().await?;
        Self::from_string(body, Some(base_url))
    }

    #[allow(dead_code)]
    pub async fn from_file(path: PathBuf, base_url: Option<Url>) -> Result<Self, Error> {
        let bytes = tokio::fs::read(path).await?;
        let body = std::str::from_utf8(&bytes).unwrap();
        Self::from_string(body.to_owned(), base_url)
    }

    fn from_string(body: String, base_url: Option<Url>) -> Result<Self, Error> {
        let mut reader = quick_xml::Reader::from_str(&body);
        reader.check_end_names(false);
        reader.trim_markup_names_in_closing_tags(true);

        let mut icons = Self::from_reader(&mut reader, base_url.as_ref());
        if let Some(base) = base_url {
            let ico_url = base.join("favicon.ico")?;
            if !icons
                .iter()
                .any(|icon| icon.is_url() && icon.url() == &ico_url)
            {
                icons.push(Favicon::for_url(ico_url, Metadata::new(Format::Ico)));
            }
        }
        if icons.is_empty() {
            return Err(Error::NoResults);
        }
        Ok(Self(icons))
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub async fn find_size(&self, size: u32) -> Option<&Favicon> {
        debug!("Trying to find icon size {size}");
        for favicon in self.0.iter() {
            if let Some(current_size) = favicon.size().await {
                // Only store the width & assumes it has the same height here to simplify things
                if current_size.0 == size {
                    return Some(favicon);
                }
            }
        }
        None
    }

    pub async fn find_best(&self) -> Option<&Favicon> {
        let mut largest_size = 0;
        let mut best = None;
        for favicon in self.0.iter() {
            if let Some(size) = favicon.size().await {
                // Only store the width & assumes it has the same height here to simplify things
                if size.0 > largest_size {
                    largest_size = size.0;
                    best = Some(favicon);
                }
            }
        }
        best.or_else(|| self.0.get(0))
    }

    fn from_reader(reader: &mut quick_xml::Reader<&[u8]>, base_url: Option<&Url>) -> Vec<Favicon> {
        let mut buf = Vec::new();
        let mut urls = Vec::new();
        loop {
            match reader.read_event(&mut buf) {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => match e.name() {
                    b"link" => {
                        if let Some(url) = Self::from_link(e, base_url) {
                            urls.push(url);
                        }
                    }
                    b"meta" => {
                        if let Some(url) = Self::from_meta(e, base_url) {
                            urls.push(url);
                        }
                    }
                    _ => (),
                },
                Ok(Event::Eof) => break,
                Err(e) => debug!("Error at position {}: {:?}", reader.buffer_position(), e),
                _ => (),
            }
        }
        buf.clear();
        urls
    }

    fn from_meta(e: &BytesStart, base_url: Option<&Url>) -> Option<Favicon> {
        let mut url = None;

        let mut has_proper_meta = false;
        for attr in e.html_attributes() {
            match attr {
                Ok(Attribute {
                    key: b"content",
                    value,
                }) => {
                    let mut href = String::from_utf8(value.into_owned()).unwrap();
                    if href.starts_with("//") {
                        href = format!("https:{}", href);
                    }
                    url = match Url::parse(&href) {
                        Ok(url) => Some(url),
                        Err(url::ParseError::RelativeUrlWithoutBase) => {
                            base_url.and_then(|base| base.join(&href).ok())
                        }
                        Err(_) => None,
                    };
                }
                Ok(Attribute {
                    key: b"name",
                    value,
                }) => {
                    if SUPPORTED_META.contains(&value.into_owned().as_slice()) {
                        has_proper_meta = true;
                    }
                }
                _ => (),
            }
            if has_proper_meta {
                if let Some(u) = url {
                    let ext = Format::from_url(&u);
                    return Some(Favicon::for_url(u, Metadata::new(ext)));
                }
            }
        }
        None
    }

    fn from_link(e: &BytesStart, base_url: Option<&Url>) -> Option<Favicon> {
        let mut data = None;
        let mut icon_url = None;
        let mut metadata = Metadata::default();

        let mut has_proper_rel = false;

        for attr in e.html_attributes() {
            match attr {
                Ok(Attribute {
                    key: b"href",
                    value,
                }) => {
                    let mut href = String::from_utf8(value.into_owned()).unwrap();
                    if href.starts_with("data:") {
                        // only bitmap icons contain ';' as a separator, svgs uses ','
                        let mut icon_data = if href.contains(';') {
                            href.trim_start_matches("data:").split(';')
                        } else {
                            href.trim_start_matches("data:").split(',')
                        };

                        let favicon_format = icon_data
                            .next()
                            .map(Format::from_mimetype)
                            .unwrap_or_default();
                        data = icon_data
                            .next()
                            .map(|data| {
                                if data.starts_with("base64") {
                                    base64::decode(data.trim_start_matches("base64,")).ok()
                                } else {
                                    Some(
                                        percent_decode_str(data)
                                            .decode_utf8()
                                            .ok()?
                                            .as_bytes()
                                            .to_vec(),
                                    )
                                }
                            })
                            .flatten();
                        metadata.format = favicon_format;
                    } else {
                        if href.starts_with("//") {
                            href = format!("https:{}", href);
                        }
                        match Url::parse(&href) {
                            Ok(url) => {
                                metadata.format = Format::from_url(&url);
                                icon_url = Some(url);
                            }
                            Err(url::ParseError::RelativeUrlWithoutBase) => {
                                base_url.and_then(|base| {
                                    base.join(&href).ok().map(|url| {
                                        metadata.format = Format::from_url(&url);
                                        icon_url = Some(url);
                                    })
                                });
                            }
                            Err(_) => (),
                        };
                    }
                }
                Ok(Attribute {
                    key: b"sizes",
                    value,
                }) => {
                    let size_inner = String::from_utf8(value.into_owned())
                        .unwrap()
                        .to_lowercase();
                    let mut size_inner = size_inner.split('x');
                    let width = size_inner.next().and_then(|w| w.parse::<u32>().ok());
                    let height = size_inner.next().and_then(|h| h.parse::<u32>().ok());
                    if let (Some(w), Some(h)) = (width, height) {
                        metadata.size = Some((w, h));
                    }
                }
                Ok(Attribute { key: b"rel", value }) => {
                    if SUPPORTED_RELS.contains(&value.into_owned().as_slice()) {
                        has_proper_rel = true;
                    }
                }
                _ => (),
            }
            if has_proper_rel && (data.is_some() || icon_url.is_some()) {
                if let Some(data) = data {
                    return Some(Favicon::for_data(data, metadata));
                } else if let Some(url) = icon_url {
                    return Some(Favicon::for_url(url, metadata));
                }
            }
        }
        None
    }
}

impl fmt::Debug for Scrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(&self.0).finish()
    }
}

impl std::ops::Index<usize> for Scrapper {
    type Output = Favicon;

    fn index(&self, idx: usize) -> &Self::Output {
        &self.0[idx]
    }
}

impl std::iter::IntoIterator for Scrapper {
    type Item = Favicon;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
