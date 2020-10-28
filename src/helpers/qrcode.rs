use anyhow::Result;
use ashpd::desktop::screenshot::{Screenshot, ScreenshotOptions, ScreenshotProxy};
use ashpd::{zbus, RequestProxy, Response, WindowIdentifier};
use gio::FileExt;
use image::GenericImageView;
use std::convert::TryFrom;
use url::Url;
use zbar_rust::ZBarImageScanner;

#[derive(Debug)]
pub struct OtpAuth {
    pub issuer: Option<String>,
    pub account: Option<String>,
    pub token: String,
}

impl TryFrom<Url> for OtpAuth {
    type Error = anyhow::Error;

    fn try_from(url: Url) -> Result<Self, Self::Error> {
        let scheme = url.scheme();
        if scheme == "otpauth" {
            let mut token = None;
            let mut issuer = None;
            for (key, val) in url.query_pairs() {
                if key == "issuer" {
                    issuer = Some(val.to_string());
                } else if key == "secret" {
                    token = Some(val.to_string());
                }
            }
            return Ok(Self {
                issuer,
                account: url.path().split(":").last().map(|c| c.to_string()),
                token: token
                    .ok_or_else(|| anyhow::format_err!("Invalid otpauth, a token is required"))?,
            });
        }
        anyhow::bail!("Invalid scheme {}", scheme)
    }
}

pub(crate) fn scan(screenshot: &gio::File) -> Result<OtpAuth> {
    let (data, _) = screenshot.load_contents(gio::NONE_CANCELLABLE)?;

    let img = image::load_from_memory(&data)?;

    let (width, height) = img.dimensions();
    let img_data: Vec<u8> = img.to_luma().to_vec();

    let mut scanner = ZBarImageScanner::new();

    let results = scanner
        .scan_y800(&img_data, width, height)
        .map_err(|e| anyhow::format_err!(e))?;

    if let Some(ref result) = results.get(0) {
        let url = Url::parse(&String::from_utf8(result.data.clone())?)?;
        return Ok(OtpAuth::try_from(url)?);
    }
    anyhow::bail!("Invalid QR code")
}

pub(crate) fn screenshot_area<F: FnOnce(String)>(callback: F) -> Result<()> {
    let connection = zbus::Connection::new_session()?;
    let proxy = ScreenshotProxy::new(&connection)?;
    let handle = proxy.screenshot(
        WindowIdentifier::default(),
        ScreenshotOptions::default().interactive(true).modal(true),
    )?;
    let request = RequestProxy::new(&connection, &handle)?;
    request.on_response(move |response: Response<Screenshot>| {
        if let Ok(screenshot) = response {
            callback(screenshot.uri);
        }
    });
    Ok(())
}
