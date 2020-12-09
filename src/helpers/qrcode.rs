use crate::models::OTPUri;
use anyhow::Result;
use ashpd::desktop::screenshot::{Screenshot, ScreenshotOptions, ScreenshotProxy};
use ashpd::{zbus, RequestProxy, Response, WindowIdentifier};
use gio::FileExt;
use image::GenericImageView;
use std::str::FromStr;
use zbar_rust::ZBarImageScanner;

pub(crate) fn scan(screenshot: &gio::File) -> Result<OTPUri> {
    let (data, _) = screenshot.load_contents(gio::NONE_CANCELLABLE)?;

    let img = image::load_from_memory(&data)?;

    let (width, height) = img.dimensions();
    let img_data: Vec<u8> = img.to_luma8().to_vec();

    let mut scanner = ZBarImageScanner::new();

    let results = scanner
        .scan_y800(&img_data, width, height)
        .map_err(|e| anyhow::format_err!(e))?;

    if let Some(ref result) = results.get(0) {
        let uri = String::from_utf8(result.data.clone())?;
        return Ok(OTPUri::from_str(&uri)?);
    }
    anyhow::bail!("Invalid QR code")
}

pub(crate) fn screenshot_area<F: FnOnce(gio::File)>(
    window: gtk::Window,
    callback: F,
) -> Result<()> {
    let connection = zbus::Connection::new_session()?;
    let proxy = ScreenshotProxy::new(&connection)?;
    let handle = proxy.screenshot(
        WindowIdentifier::from(window),
        ScreenshotOptions::default().interactive(true).modal(true),
    )?;
    let request = RequestProxy::new(&connection, &handle)?;
    request.on_response(move |response: Response<Screenshot>| {
        if let Ok(screenshot) = response {
            callback(gio::File::new_for_uri(&screenshot.uri));
        }
    })?;
    Ok(())
}
