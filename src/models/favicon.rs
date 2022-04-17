use super::CLIENT;
use image::io::Reader as ImageReader;
use once_cell::sync::Lazy;
use percent_encoding::percent_decode_str;
use quick_xml::events::{attributes::Attribute, BytesStart, Event};
use std::{fmt, io::Cursor, path::PathBuf};
use tokio::io::AsyncWriteExt;
use url::Url;

pub static FAVICONS_PATH: Lazy<std::path::PathBuf> = Lazy::new(|| {
    gtk::glib::user_cache_dir()
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

const SUPPORTED_META: [&[u8]; 1] = [b"msapplication-TileImage"];

#[derive(Debug, PartialEq, Eq)]
pub enum FaviconType {
    Png,
    Svg,
    Ico,
}

impl FaviconType {
    /// Convert a file extension to a FaviconType and default to png if none can be
    /// detected
    pub fn from_url(url: &Url) -> Self {
        let ext = std::path::Path::new(url.path())
            .extension()
            .map(|e| e.to_str().unwrap());
        match ext {
            Some("png") => FaviconType::Png,
            Some("ico") => FaviconType::Ico,
            Some("svg") => FaviconType::Svg,
            _ => Self::default(),
        }
    }

    pub fn from_mimetype(mimetype: &str) -> Self {
        match mimetype {
            "image/x-icon" => FaviconType::Ico,
            "image/png" => FaviconType::Png,
            "image/svg+xml" => FaviconType::Svg,
            _ => Self::default(),
        }
    }
}

impl fmt::Display for FaviconType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Png => f.write_str("png"),
            Self::Ico => f.write_str("ico"),
            Self::Svg => f.write_str("svg"),
        }
    }
}

impl Default for FaviconType {
    fn default() -> Self {
        Self::Png
    }
}

#[derive(PartialEq)]
pub enum Favicon {
    Data(Vec<u8>, FaviconType),
    Url(Url, FaviconType),
}

impl fmt::Debug for Favicon {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Data(bytes, f_type) => f
                .debug_struct("Favicon")
                .field("data", bytes)
                .field("type", f_type)
                .finish(),
            Self::Url(url, f_type) => f
                .debug_struct("Favicon")
                .field("url", &url.as_str())
                .field("type", f_type)
                .finish(),
        }
    }
}

impl Favicon {
    #[allow(dead_code)]
    pub fn is_data(&self) -> bool {
        matches!(self, Self::Data(_, _))
    }

    #[allow(dead_code)]
    pub fn is_url(&self) -> bool {
        matches!(self, Self::Url(_, _))
    }

    pub fn mime_type(&self) -> &FaviconType {
        match self {
            Self::Data(_, f_type) => f_type,
            Self::Url(_, f_type) => f_type,
        }
    }

    pub async fn cache(&self, icon_name: &str) -> anyhow::Result<PathBuf> {
        let body = match self {
            Self::Data(bytes, _) => bytes.to_owned(),
            Self::Url(url, _) => {
                let res = CLIENT.get(url.as_str()).send().await?;
                res.bytes().await?.to_vec()
            }
        };

        if self.mime_type() == &FaviconType::Ico {
            log::debug!("Found a .ico favicon, converting to PNG");

            let cache_path = FAVICONS_PATH.join(format!("{}.png", icon_name));
            let mut dest = tokio::fs::File::create(cache_path.clone()).await?;

            if let Ok(ico) = image::load_from_memory_with_format(&body, image::ImageFormat::Ico) {
                let mut cursor = std::io::Cursor::new(vec![]);
                ico.write_to(&mut cursor, image::ImageOutputFormat::Png)?;
                dest.write_all(cursor.get_ref()).await?;
            } else {
                log::debug!("It seems to not be a .ICO favicon, fallback to PNG");
                dest.write_all(&body).await?;
            };

            Ok(cache_path)
        } else {
            let cache_path = FAVICONS_PATH.join(icon_name);
            let mut dest = tokio::fs::File::create(cache_path.clone()).await?;
            dest.write_all(&body).await?;

            Ok(cache_path)
        }
    }

    async fn size(&self) -> Option<(u32, u32)> {
        match self {
            Self::Data(data, f_type) => {
                if f_type == &FaviconType::Svg {
                    self.svg_dimensions(std::str::from_utf8(data).ok().unwrap())
                } else {
                    self.bitmap_dimensions(data, f_type)
                }
            }
            Self::Url(url, f_type) => {
                let response = CLIENT.get(url.as_str()).send().await.ok()?;

                if f_type == &FaviconType::Svg {
                    self.svg_dimensions(&response.text().await.ok()?)
                } else {
                    let bytes = response.bytes().await.ok()?;
                    self.bitmap_dimensions(&bytes, f_type)
                }
            }
        }
    }

    fn svg_dimensions(&self, svg: &str) -> Option<(u32, u32)> {
        let metadata = svg_metadata::Metadata::parse(svg).ok()?;

        let width = metadata.width()? as u32;
        let height = metadata.height()? as u32;
        Some((width, height))
    }

    fn bitmap_dimensions(&self, body: &[u8], format: &FaviconType) -> Option<(u32, u32)> {
        let mut image = ImageReader::new(Cursor::new(body));

        let format = image::ImageFormat::from_extension(format.to_string())?;
        image.set_format(format);
        image.into_dimensions().ok()
    }
}

#[derive(Debug)]
pub enum FaviconError {
    Reqwest(reqwest::Error),
    Url(url::ParseError),
    Io(std::io::Error),
    NoResults,
}

impl From<std::io::Error> for FaviconError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<reqwest::Error> for FaviconError {
    fn from(e: reqwest::Error) -> Self {
        Self::Reqwest(e)
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
pub struct FaviconScrapper(Vec<Favicon>);

impl FaviconScrapper {
    pub async fn from_url(base_url: Url) -> Result<Self, FaviconError> {
        let res = CLIENT.get(base_url.as_str())
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 11_1) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/14.0.2 Safari/605.1.15")
            .send()
            .await?;
        let body = res.text().await?;
        Self::from_string(body, Some(base_url))
    }

    #[allow(dead_code)]
    pub async fn from_file(path: PathBuf, base_url: Option<Url>) -> Result<Self, FaviconError> {
        let bytes = tokio::fs::read(path).await?;
        let body = std::str::from_utf8(&bytes).unwrap();
        Self::from_string(body.to_owned(), base_url)
    }

    fn from_string(body: String, base_url: Option<Url>) -> Result<Self, FaviconError> {
        let mut reader = quick_xml::Reader::from_str(&body);
        reader.check_end_names(false);
        reader.trim_markup_names_in_closing_tags(true);

        let mut icons = Self::from_reader(&mut reader, base_url.as_ref());
        if let Some(base) = base_url {
            icons.push(Favicon::Url(base.join("favicon.ico")?, FaviconType::Ico));
        }
        if icons.is_empty() {
            return Err(FaviconError::NoResults);
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
            if has_proper_meta && url.is_some() {
                break;
            }
        }
        if has_proper_meta {
            if let Some(u) = url {
                let ext = FaviconType::from_url(&u);
                return Some(Favicon::Url(u, ext));
            }
        }
        None
    }

    fn from_link(e: &BytesStart, base_url: Option<&Url>) -> Option<Favicon> {
        let mut url = None;

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
                        let mut icon_data = if href.contains(";") {
                            href.trim_start_matches("data:").split(';')
                        } else {
                            href.trim_start_matches("data:").split(',')
                        };

                        let favicon_type = icon_data
                            .next()
                            .map(FaviconType::from_mimetype)
                            .unwrap_or_default();
                        let data = icon_data
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

                        url = data.map(|d| Favicon::Data(d, favicon_type));
                    } else {
                        if href.starts_with("//") {
                            href = format!("https:{}", href);
                        }
                        url = match Url::parse(&href) {
                            Ok(url) => {
                                let ext = FaviconType::from_url(&url);
                                Some(Favicon::Url(url, ext))
                            }
                            Err(url::ParseError::RelativeUrlWithoutBase) => {
                                base_url.and_then(|base| {
                                    base.join(&href).ok().map(|u| {
                                        let ext = FaviconType::from_url(&u);
                                        Favicon::Url(u, ext)
                                    })
                                })
                            }
                            Err(_) => None,
                        };
                    }
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

#[cfg(test)]
mod tests {
    use super::FaviconScrapper;
    use super::Url;
    use super::{Favicon, FaviconType};

    #[tokio::test]
    async fn from_file() {
        let base_url = Url::parse("https://github.com").unwrap();
        let expected_output = Favicon::Url(
            Url::parse("https://github.githubassets.com/favicon.ico").unwrap(),
            FaviconType::Ico,
        );

        let scrapper = FaviconScrapper::from_file(
            "./tests/favicon/url_shortcut_icon_link.html".into(),
            Some(base_url.clone()),
        )
        .await
        .unwrap();
        let best = scrapper.find_best().await;
        assert_eq!(best, Some(&expected_output));

        let scrapper = FaviconScrapper::from_file(
            "./tests/favicon/url_icon_link.html".into(),
            Some(base_url.clone()),
        )
        .await
        .unwrap();
        let best = scrapper.find_best().await;
        assert_eq!(best, Some(&expected_output));

        let scrapper = FaviconScrapper::from_file(
            "./tests/favicon/url_fluid_icon.html".into(),
            Some(base_url.clone()),
        )
        .await
        .unwrap();
        let best = scrapper.find_best().await;
        assert_eq!(best, Some(&expected_output));

        let scrapper = FaviconScrapper::from_file(
            "./tests/favicon/url_apple_touch_icon_precomposed_link.html".into(),
            Some(base_url.clone()),
        )
        .await
        .unwrap();
        let best = scrapper.find_best().await;
        assert_eq!(best, Some(&expected_output));

        let scrapper = FaviconScrapper::from_file(
            "./tests/favicon/url_apple_touch_icon.html".into(),
            Some(base_url.clone()),
        )
        .await
        .unwrap();
        let best = scrapper.find_best().await;
        assert_eq!(best, Some(&expected_output));
    }

    #[tokio::test]
    async fn meta_tag() {
        let base_url = Url::parse("https://gitlab.com").unwrap();
        let expected_output = Favicon::Url(Url::parse("https://assets.gitlab-static.net/assets/msapplication-tile-1196ec67452f618d39cdd85e2e3a542f76574c071051ae7effbfde01710eb17d.png").unwrap(), FaviconType::Png);
        let scrapper =
            FaviconScrapper::from_file("./tests/favicon/meta_tag.html".into(), Some(base_url))
                .await
                .unwrap();
        let best = scrapper.find_best().await;
        assert_eq!(best, Some(&expected_output));
    }

    #[tokio::test]
    async fn url_with_port() {
        let base_url = Url::parse("http://127.0.0.1:8000/index.html").unwrap();
        let expected_output = Favicon::Url(
            Url::parse("http://127.0.0.1:8000/favicon.ico").unwrap(),
            FaviconType::Ico,
        );
        let scrapper =
            FaviconScrapper::from_file("./tests/favicon/url_with_port.html".into(), Some(base_url))
                .await
                .unwrap();
        let best = scrapper.find_best().await;
        assert_eq!(best, Some(&expected_output));
    }

    #[tokio::test]
    async fn data_base64() {
        let scrapper = FaviconScrapper::from_file("./tests/favicon/data_base64.html".into(), None)
            .await
            .unwrap();
        assert!(!scrapper.is_empty());
        assert_eq!(scrapper.len(), 1);
        let best = scrapper.find_best().await.unwrap();

        assert_eq!(best.mime_type(), &FaviconType::Ico);
        assert!(best.is_data());
        assert_eq!(best.size().await, Some((16, 16)));
    }

    #[tokio::test]
    async fn data_svg() {
        let scrapper = FaviconScrapper::from_file("./tests/favicon/data_svg.html".into(), None)
            .await
            .unwrap();
        assert!(!scrapper.is_empty());
        assert_eq!(scrapper.len(), 1);
        let best = scrapper.find_best().await.unwrap();

        assert_eq!(best.mime_type(), &FaviconType::Svg);
        assert!(best.is_data());
    }
}
