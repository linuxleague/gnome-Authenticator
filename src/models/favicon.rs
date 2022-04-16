use super::CLIENT;
use image::io::Reader as ImageReader;
use once_cell::sync::Lazy;
use quick_xml::events::{attributes::Attribute, BytesStart, Event};
use std::{io::Cursor, path::PathBuf};
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

pub struct Favicon(Vec<Url>);

impl std::fmt::Debug for Favicon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list()
            .entries(self.0.iter().map(|u| u.as_str()))
            .finish()
    }
}

impl Favicon {
    pub async fn find_best(&self) -> Option<&Url> {
        let mut largest_size = 0;
        let mut best = None;
        for url in self.0.iter() {
            if let Some(size) = self.size(url).await {
                // Only store the width & assumes it has the same height here to simplify things
                if size.0 > largest_size {
                    largest_size = size.0;
                    best = Some(url);
                }
            }
        }
        best.or_else(|| self.0.get(0))
    }

    pub async fn size(&self, url: &Url) -> Option<(u32, u32)> {
        let response = CLIENT.get(url.as_str()).send().await.ok()?;

        let ext = std::path::Path::new(url.path())
            .extension()
            .map(|e| e.to_str().unwrap())?;

        if ext == "svg" {
            let body = response.text().await.ok()?;

            self.svg_dimensions(body)
        } else {
            let bytes = response.bytes().await.ok()?;

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
    pub async fn from_url(base_url: Url) -> Result<Favicon, FaviconError> {
        let res = CLIENT.get(base_url.as_str())
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 11_1) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/14.0.2 Safari/605.1.15")
            .send()
            .await?;
        let body = res.text().await?;
        Self::from_string(base_url, body)
    }

    #[allow(dead_code)]
    pub async fn from_file(base_url: Url, path: PathBuf) -> Result<Favicon, FaviconError> {
        let bytes = tokio::fs::read(path).await?;
        let body = std::str::from_utf8(&bytes).unwrap();
        Self::from_string(base_url, body.to_owned())
    }

    fn from_string(base_url: Url, body: String) -> Result<Favicon, FaviconError> {
        let mut reader = quick_xml::Reader::from_str(&body);
        reader.check_end_names(false);
        reader.trim_markup_names_in_closing_tags(true);

        let mut icons = Self::from_reader(&mut reader, &base_url);
        icons.push(base_url.join("favicon.ico")?);
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

    fn from_meta(e: &BytesStart, base_url: &Url) -> Option<Url> {
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
                        Err(url::ParseError::RelativeUrlWithoutBase) => base_url.join(&href).ok(),
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
            return url;
        }
        None
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

#[cfg(test)]
mod tests {
    use super::FaviconScrapper;
    use super::Url;

    #[tokio::test]
    async fn from_file() {
        let base_url = Url::parse("https://github.com").unwrap();
        let scrapper = FaviconScrapper::from_file(
            base_url.clone(),
            "./tests/favicon/url_shortcut_icon_link.html".into(),
        )
        .await
        .unwrap();
        let best = scrapper.find_best().await;
        assert_eq!(
            best,
            Url::parse("https://github.githubassets.com/favicon.ico")
                .ok()
                .as_ref()
        );

        let scrapper = FaviconScrapper::from_file(
            base_url.clone(),
            "./tests/favicon/url_icon_link.html".into(),
        )
        .await
        .unwrap();
        let best = scrapper.find_best().await;
        assert_eq!(
            best,
            Url::parse("https://github.githubassets.com/favicon.ico")
                .ok()
                .as_ref()
        );

        let scrapper = FaviconScrapper::from_file(
            base_url.clone(),
            "./tests/favicon/url_fluid_icon.html".into(),
        )
        .await
        .unwrap();
        let best = scrapper.find_best().await;
        assert_eq!(
            best,
            Url::parse("https://github.githubassets.com/favicon.ico")
                .ok()
                .as_ref()
        );

        let scrapper = FaviconScrapper::from_file(
            base_url.clone(),
            "./tests/favicon/url_apple_touch_icon_precomposed_link.html".into(),
        )
        .await
        .unwrap();
        let best = scrapper.find_best().await;
        assert_eq!(
            best,
            Url::parse("https://github.githubassets.com/favicon.ico")
                .ok()
                .as_ref()
        );

        let scrapper = FaviconScrapper::from_file(
            base_url.clone(),
            "./tests/favicon/url_apple_touch_icon.html".into(),
        )
        .await
        .unwrap();
        let best = scrapper.find_best().await;
        assert_eq!(
            best,
            Url::parse("https://github.githubassets.com/favicon.ico")
                .ok()
                .as_ref()
        );
    }

    #[tokio::test]
    async fn meta_tag() {
        let base_url = Url::parse("https://gitlab.com").unwrap();
        let scrapper =
            FaviconScrapper::from_file(base_url.clone(), "./tests/favicon/meta_tag.html".into())
                .await
                .unwrap();
        let best = scrapper.find_best().await;
        assert_eq!(best, Url::parse("https://assets.gitlab-static.net/assets/msapplication-tile-1196ec67452f618d39cdd85e2e3a542f76574c071051ae7effbfde01710eb17d.png").ok().as_ref());
    }
}
