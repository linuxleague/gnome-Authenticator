use crate::Format;

#[derive(Debug, Default, PartialEq)]
/// Favicon metadata.
pub struct Metadata {
    pub(crate) format: Format,
    pub(crate) size: Option<(u32, u32)>,
}

impl Metadata {
    pub(crate) fn new(format: Format) -> Self {
        Self { format, size: None }
    }

    #[allow(dead_code)]
    pub(crate) fn with_size(format: Format, size: (u32, u32)) -> Self {
        Self {
            format,
            size: Some(size),
        }
    }

    /// The favicon's image format.
    pub fn format(&self) -> &Format {
        &self.format
    }

    /// The favicon's size if was specified in the HTML tags.
    pub fn size(&self) -> Option<(u32, u32)> {
        self.size
    }
}
