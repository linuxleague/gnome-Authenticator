use std::{fmt, io::Cursor, path::PathBuf};

use image::io::Reader as ImageReader;
use tokio::{io::AsyncWriteExt, sync::Mutex};
use url::Url;

use crate::{client, Error, Format, Metadata};

pub struct Favicon {
    url: Option<Url>,
    data: Mutex<Option<Vec<u8>>>,
    metadata: Metadata,
    is_url: bool,
}

impl Favicon {
    pub(crate) fn for_url<U: reqwest::IntoUrl>(url: U, metadata: Metadata) -> Self {
        Self {
            url: Some(url.into_url().expect("Favicon expects a valid url")),
            metadata,
            data: Default::default(),
            is_url: true,
        }
    }

    pub(crate) fn for_data(data: Vec<u8>, metadata: Metadata) -> Self {
        Self {
            url: None,
            metadata,
            data: Mutex::new(Some(data)),
            is_url: false,
        }
    }

    #[allow(dead_code)]
    pub fn is_data(&self) -> bool {
        !self.is_url
    }

    #[allow(dead_code)]
    pub fn is_url(&self) -> bool {
        self.is_url
    }

    /// Returns the favicon's URL
    ///
    /// # Panics
    ///
    /// If the favicon contains the data instead of a URL, you are supposed to
    /// check it content using [`Favicon::is_url`]
    pub fn url(&self) -> &Url {
        match &self.url {
            Some(url) => url,
            _ => panic!("Favicon contains the data not a url"),
        }
    }

    /// Returns the favicon's data
    pub async fn data(&self) -> Result<Vec<u8>, Error> {
        let mut lock = self.data.lock().await;
        if self.is_data() {
            Ok(lock.as_ref().unwrap().clone())
        } else {
            let has_cached_data = lock.is_some();
            if !has_cached_data {
                let res = client().get(self.url().as_str()).send().await?;
                let bytes = res.bytes().await?.to_vec();
                lock.replace(bytes);
            }
            Ok(lock.as_ref().unwrap().clone())
        }
    }

    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    /// Save the favicon into `destination` and convert it to a [`Format::Png`]
    /// if it is original format is [`Format::Ico`].
    pub async fn save(&self, destination: PathBuf) -> Result<(), Error> {
        tracing::debug!("Caching the icon into {:#?}", destination);
        let format = *self.metadata().format();
        let body = self.data().await?;
        if format.is_ico() {
            tracing::debug!("Found a ICO favicon, converting to PNG");
            if let Ok(ico) = image::load_from_memory_with_format(&body, image::ImageFormat::Ico) {
                ico.save_with_format(destination, image::ImageFormat::Png)?;
                return Ok(());
            } else {
                tracing::debug!("It seems to not be a ICO favicon, fallback to PNG");
            };
        }
        let mut dest = tokio::fs::File::create(destination).await?;
        dest.write_all(&body).await?;
        Ok(())
    }

    pub async fn size(&self) -> Option<(u32, u32)> {
        let size = self.metadata().size();
        let format = *self.metadata.format();
        if let Some(size) = size {
            Some(size)
        } else {
            let body = self.data().await.ok()?;
            if format.is_svg() {
                Favicon::svg_dimensions(std::str::from_utf8(&body).ok().unwrap())
            } else {
                Favicon::bitmap_dimensions(&body, &format)
            }
        }
    }

    fn svg_dimensions(svg: &str) -> Option<(u32, u32)> {
        let metadata = svg_metadata::Metadata::parse(svg).ok()?;

        let width = metadata.width()? as u32;
        let height = metadata.height()? as u32;
        Some((width, height))
    }

    fn bitmap_dimensions(body: &[u8], format: &Format) -> Option<(u32, u32)> {
        let mut image = ImageReader::new(Cursor::new(body));

        let format = image::ImageFormat::from_extension(format.to_string())?;
        image.set_format(format);
        image.into_dimensions().ok()
    }
}

impl fmt::Debug for Favicon {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let metadata = self.metadata();
        if self.is_data() {
            f.debug_struct("Favicon")
                .field("metadata", &metadata)
                .finish()
        } else {
            f.debug_struct("Favicon")
                .field("url", &self.url().as_str())
                .field("metadata", &metadata)
                .finish()
        }
    }
}

impl PartialEq for Favicon {
    fn eq(&self, other: &Self) -> bool {
        self.is_url == other.is_url && self.metadata == other.metadata && self.url == other.url
    }
}
