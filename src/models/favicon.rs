use super::CLIENT;
use image::io::Reader as ImageReader;
use once_cell::sync::Lazy;
use percent_encoding::percent_decode_str;
use quick_xml::events::{attributes::Attribute, BytesStart, Event};
use reqwest::IntoUrl;
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
pub enum Type {
    Png,
    Svg,
    Ico,
}

impl Type {
    /// Convert a file extension to a Type and default to png if none can be
    /// detected
    pub fn from_url(url: &Url) -> Self {
        let ext = std::path::Path::new(url.path())
            .extension()
            .map(|e| e.to_str().unwrap());
        match ext {
            Some("png") => Type::Png,
            Some("ico") => Type::Ico,
            Some("svg") => Type::Svg,
            _ => Self::default(),
        }
    }

    pub fn from_mimetype(mimetype: &str) -> Self {
        match mimetype {
            "image/x-icon" => Type::Ico,
            "image/png" => Type::Png,
            "image/svg+xml" => Type::Svg,
            _ => Self::default(),
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Png => f.write_str("png"),
            Self::Ico => f.write_str("ico"),
            Self::Svg => f.write_str("svg"),
        }
    }
}

impl Default for Type {
    fn default() -> Self {
        Self::Png
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct Metadata {
    type_: Type,
    size: Option<(u32, u32)>,
}

impl Metadata {
    pub fn new(type_: Type) -> Self {
        Self { type_, size: None }
    }

    #[allow(dead_code)]
    pub fn with_size(type_: Type, size: (u32, u32)) -> Self {
        Self {
            type_,
            size: Some(size),
        }
    }

    pub fn type_(&self) -> &Type {
        &self.type_
    }

    pub fn size(&self) -> Option<(u32, u32)> {
        self.size
    }
}

#[derive(PartialEq)]
pub enum Favicon {
    Data(Vec<u8>, Metadata),
    Url(Url, Metadata),
}

impl fmt::Debug for Favicon {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Data(_, metadata) => f
                .debug_struct("Favicon")
                .field("type", &metadata.type_())
                .field("size", &metadata.size())
                .finish(),
            Self::Url(url, metadata) => f
                .debug_struct("Favicon")
                .field("url", &url.as_str())
                .field("type", &metadata.type_())
                .field("size", &metadata.size())
                .finish(),
        }
    }
}

impl Favicon {
    pub fn for_url<U: IntoUrl>(url: U, metadata: Metadata) -> Self {
        Self::Url(url.into_url().expect("Invalid URL"), metadata)
    }

    pub fn for_data(data: Vec<u8>, metadata: Metadata) -> Self {
        Self::Data(data, metadata)
    }

    #[allow(dead_code)]
    pub fn is_data(&self) -> bool {
        matches!(self, Self::Data(_, _))
    }

    #[allow(dead_code)]
    pub fn is_url(&self) -> bool {
        matches!(self, Self::Url(_, _))
    }

    pub fn metadata(&self) -> &Metadata {
        match self {
            Self::Data(_, metadata) => metadata,
            Self::Url(_, metadata) => metadata,
        }
    }

    pub async fn cache(&self, icon_name: &str) -> anyhow::Result<()> {
        let body = match self {
            Self::Data(bytes, _) => bytes.to_owned(),
            Self::Url(url, _) => {
                let res = CLIENT.get(url.as_str()).send().await?;
                res.error_for_status_ref()?;
                res.bytes().await?.to_vec()
            }
        };
        let cache_path = FAVICONS_PATH.join(icon_name);
        let mut dest = tokio::fs::File::create(cache_path.clone()).await?;

        if self.metadata().type_() == &Type::Ico {
            log::debug!("Found a .ico favicon, converting to PNG");
            if let Ok(ico) = image::load_from_memory_with_format(&body, image::ImageFormat::Ico) {
                let mut cursor = std::io::Cursor::new(vec![]);
                ico.write_to(&mut cursor, image::ImageOutputFormat::Png)?;
                dest.write_all(cursor.get_ref()).await?;
                return Ok(());
            } else {
                log::debug!("It seems to not be a .ICO favicon, fallback to PNG");
            };
        }
        dest.write_all(&body).await?;
        Ok(())
    }

    async fn size(&self) -> Option<(u32, u32)> {
        let type_ = self.metadata().type_();
        match self {
            Self::Data(data, metadata) => metadata.size().or_else(|| {
                if type_ == &Type::Svg {
                    self.svg_dimensions(std::str::from_utf8(data).ok().unwrap())
                } else {
                    self.bitmap_dimensions(data, type_)
                }
            }),
            Self::Url(url, metadata) => {
                if let Some(size) = metadata.size() {
                    Some(size)
                } else {
                    let response = CLIENT.get(url.as_str()).send().await.ok()?;
                    response.error_for_status_ref().ok()?;
                    if type_ == &Type::Svg {
                        self.svg_dimensions(&response.text().await.ok()?)
                    } else {
                        let bytes = response.bytes().await.ok()?;
                        self.bitmap_dimensions(&bytes, type_)
                    }
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

    fn bitmap_dimensions(&self, body: &[u8], format: &Type) -> Option<(u32, u32)> {
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

pub struct FaviconScrapper(Vec<Favicon>);

impl FaviconScrapper {
    pub async fn from_url(base_url: Url) -> Result<Self, FaviconError> {
        let res = CLIENT.get(base_url.as_str())
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 11_1) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/14.0.2 Safari/605.1.15")
            .send()
            .await?;
        res.error_for_status_ref()?;
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
            icons.push(Favicon::for_url(
                base.join("favicon.ico")?,
                Metadata::new(Type::Ico),
            ));
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

    pub async fn find_size(&self, size: u32) -> Option<&Favicon> {
        let mut best = None;
        for favicon in self.0.iter() {
            if let Some(current_size) = favicon.size().await {
                // Only store the width & assumes it has the same height here to simplify things
                if current_size.0 == size {
                    best = Some(favicon);
                    break;
                }
            }
        }
        best
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
                let ext = Type::from_url(&u);
                return Some(Favicon::Url(u, Metadata::new(ext)));
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

                        let favicon_type = icon_data
                            .next()
                            .map(Type::from_mimetype)
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
                        metadata.type_ = favicon_type;
                    } else {
                        if href.starts_with("//") {
                            href = format!("https:{}", href);
                        }
                        match Url::parse(&href) {
                            Ok(url) => {
                                metadata.type_ = Type::from_url(&url);
                                icon_url = Some(url);
                            }
                            Err(url::ParseError::RelativeUrlWithoutBase) => {
                                base_url.and_then(|base| {
                                    base.join(&href).ok().map(|url| {
                                        metadata.type_ = Type::from_url(&url);
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
                break;
            }
        }
        if has_proper_rel {
            if let Some(data) = data {
                return Some(Favicon::for_data(data, metadata));
            } else if let Some(url) = icon_url {
                return Some(Favicon::for_url(url, metadata));
            }
        }
        None
    }
}

impl fmt::Debug for FaviconScrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(&self.0).finish()
    }
}

impl std::ops::Index<usize> for FaviconScrapper {
    type Output = Favicon;

    fn index(&self, idx: usize) -> &Self::Output {
        &self.0[idx]
    }
}

#[cfg(test)]
mod tests {
    use super::FaviconScrapper;
    use super::Url;
    use super::{Favicon, Metadata, Type};

    #[tokio::test]
    async fn from_file() {
        let base_url = Url::parse("https://github.com").unwrap();
        let expected_output = Favicon::for_url(
            "https://github.githubassets.com/favicon.ico",
            Metadata::new(Type::Ico),
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
        let expected_output = Favicon::for_url("https://assets.gitlab-static.net/assets/msapplication-tile-1196ec67452f618d39cdd85e2e3a542f76574c071051ae7effbfde01710eb17d.png", Metadata::new(Type::Png));
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
        let expected_output = Favicon::for_url(
            "http://127.0.0.1:8000/favicon.ico",
            Metadata::new(Type::Ico),
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

        assert_eq!(best.metadata().type_(), &Type::Ico);
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

        assert_eq!(best.metadata().type_(), &Type::Svg);
        assert!(best.is_data());
    }

    #[tokio::test]
    async fn size() {
        let base_url = Url::parse("https://about.gitlab.com").ok();
        let scrapper = FaviconScrapper::from_file("./tests/favicon/size.html".into(), base_url)
            .await
            .unwrap();
        assert!(!scrapper.is_empty());
        // There are 16 but we always add the favicon.ico to try in case it exists as well
        assert_eq!(scrapper.len(), 16 + 1);

        assert_eq!(
            scrapper[0],
            Favicon::for_url(
                "https://about.gitlab.com/nuxt-images/ico/mstile-144x144.png?cache=20220413",
                Metadata::new(Type::Png)
            )
        );
        assert_eq!(
            scrapper[1],
            Favicon::for_url(
                "https://about.gitlab.com/nuxt-images/ico/favicon.ico?cache=20220413",
                Metadata::new(Type::Ico),
            )
        );
        assert_eq!(
            scrapper[2],
            Favicon::for_url(
                "https://about.gitlab.com/nuxt-images/ico/favicon-192x192.png?cache=2022041",
                Metadata::with_size(Type::Png, (192, 192)),
            )
        );
        assert_eq!(
            scrapper[3],
            Favicon::for_url(
                "https://about.gitlab.com/nuxt-images/ico/favicon-160x160.png?cache=2022041",
                Metadata::with_size(Type::Png, (160, 160))
            )
        );
        assert_eq!(
            scrapper[4],
            Favicon::for_url(
                "https://about.gitlab.com/nuxt-images/ico/favicon-96x96.png?cache=2022041",
                Metadata::with_size(Type::Png, (96, 96))
            )
        );
        assert_eq!(
            scrapper[5],
            Favicon::for_url(
                "https://about.gitlab.com/nuxt-images/ico/favicon-32x32.png?cache=2022041",
                Metadata::with_size(Type::Png, (32, 32))
            )
        );
        assert_eq!(
            scrapper[6],
            Favicon::for_url(
                "https://about.gitlab.com/nuxt-images/ico/favicon-16x16.png?cache=2022041",
                Metadata::with_size(Type::Png, (16, 16))
            )
        );
        assert_eq!(
            scrapper[7],
            Favicon::for_url(
                "https://about.gitlab.com/nuxt-images/ico/apple-touch-icon-57x57.png?cache=2022041",
                Metadata::with_size(Type::Png, (57, 57))
            )
        );
        assert_eq!(
            scrapper[8],
            Favicon::for_url(
                "https://about.gitlab.com/nuxt-images/ico/apple-touch-icon-60x60.png?cache=2022041",
                Metadata::with_size(Type::Png, (60, 60))
            )
        );
        assert_eq!(
            scrapper[9],
            Favicon::for_url(
                "https://about.gitlab.com/nuxt-images/ico/apple-touch-icon-72x72.png?cache=2022041",
                Metadata::with_size(Type::Png, (72, 72))
            )
        );
        assert_eq!(
            scrapper[10],
            Favicon::for_url(
                "https://about.gitlab.com/nuxt-images/ico/apple-touch-icon-76x76.png?cache=2022041",
                Metadata::with_size(Type::Png, (76, 76))
            )
        );
        assert_eq!(scrapper[11], Favicon::for_url("https://about.gitlab.com/nuxt-images/ico/apple-touch-icon-114x114.png?cache=2022041", Metadata::with_size(Type::Png, (114, 114 ))));
        assert_eq!(scrapper[12], Favicon::for_url("https://about.gitlab.com/nuxt-images/ico/apple-touch-icon-120x120.png?cache=2022041", Metadata::with_size(Type::Png, (120, 120 ))));
        assert_eq!(scrapper[13], Favicon::for_url("https://about.gitlab.com/nuxt-images/ico/apple-touch-icon-144x144.png?cache=2022041", Metadata::with_size(Type::Png, (144, 144 ))));
        assert_eq!(scrapper[14], Favicon::for_url("https://about.gitlab.com/nuxt-images/ico/apple-touch-icon-152x152.png?cache=2022041", Metadata::with_size(Type::Png, (152, 152 ))));
        assert_eq!(scrapper[15], Favicon::for_url("https://about.gitlab.com/nuxt-images/ico/apple-touch-icon-180x180.png?cache=2022041", Metadata::with_size(Type::Png, (180, 180 ))));
    }
}
