//! Offline QR recognition. Coordinates remain attached to each physical code,
//! including repeated payloads and codes that can be located but not decoded.
use image::ImageDecoder;
use serde::Serialize;

pub const MAX_PNG_BYTES: usize = 20 * 1024 * 1024;
const MAX_PIXELS: u64 = 32_000_000;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QrCode {
    pub index: usize,
    pub corners: [[f64; 2]; 4],
    pub text: Option<String>,
    pub url: Option<String>,
    pub host: Option<String>,
    pub suspicious: bool,
}

pub fn link(text: &str) -> Option<url::Url> {
    if text != text.trim()
        || text.chars().any(|c| c.is_control() || c.is_whitespace())
        || text.contains('\\')
    {
        return None;
    }
    // Require the explicit web scheme; never execute arbitrary URI handlers.
    if !text.to_ascii_lowercase().starts_with("https://")
        && !text.to_ascii_lowercase().starts_with("http://")
    {
        return None;
    }
    let url = url::Url::parse(text).ok()?;
    if !matches!(url.scheme(), "https" | "http")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return None;
    }
    Some(url)
}

pub fn describe(index: usize, corners: [[f64; 2]; 4], text: Option<String>) -> QrCode {
    let parsed = text.as_deref().and_then(link);
    let host = parsed
        .as_ref()
        .and_then(|u| u.host_str())
        .map(str::to_owned);
    let suspicious = text.as_ref().is_some_and(|text| {
        parsed.as_ref().is_some_and(|u| u.scheme() == "http")
            || host.as_ref().is_some_and(|h| {
                h.contains("xn--")
                    || h == "localhost"
                    || h.starts_with('[')
                    || h.parse::<std::net::IpAddr>().is_ok()
            })
            || (parsed.is_none() && (text.contains(":") || text.contains('\u{202e}')))
    });
    QrCode {
        index,
        corners,
        text,
        url: parsed.map(|u| u.to_string()),
        host,
        suspicious,
    }
}

pub fn decode(png: &[u8]) -> Result<(u32, u32, Vec<QrCode>), String> {
    if png.len() > MAX_PNG_BYTES {
        return Err("The QR image exceeds the size limit.".into());
    }
    let decoder = image::codecs::png::PngDecoder::new(std::io::Cursor::new(png))
        .map_err(|_| "The QR image could not be read.".to_string())?;
    let (width, height) = decoder.dimensions();
    if width == 0 || height == 0 || u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err("The QR image exceeds the size limit.".into());
    }
    let gray = image::DynamicImage::from_decoder(decoder)
        .map_err(|_| "The QR image could not be read.".to_string())?
        .into_luma8();
    let mut decoder = quircs::Quirc::default();
    let mut codes = Vec::new();
    for code in decoder
        .identify(width as usize, height as usize, &gray)
        .take(64)
        .flatten()
    {
        let corners = code.corners.map(|p| {
            [
                (p.x as f64 / width as f64).clamp(0.0, 1.0),
                (p.y as f64 / height as f64).clamp(0.0, 1.0),
            ]
        });
        let text = code
            .decode()
            .ok()
            .and_then(|d| String::from_utf8(d.payload).ok())
            .filter(|s| !s.is_empty() && s.len() <= 16 * 1024);
        codes.push(describe(codes.len(), corners, text));
    }
    codes.sort_by(|a, b| {
        a.corners[0][1]
            .total_cmp(&b.corners[0][1])
            .then(a.corners[0][0].total_cmp(&b.corners[0][0]))
    });
    for (i, code) in codes.iter_mut().enumerate() {
        code.index = i;
    }
    Ok((width, height, codes))
}

pub fn crop_code(png: &[u8], code: &QrCode) -> Result<(Vec<u8>, u32, u32), String> {
    let image = image::load_from_memory_with_format(png, image::ImageFormat::Png)
        .map_err(|_| "The QR image could not be read.".to_string())?;
    let xs = code.corners.map(|p| p[0] * image.width() as f64);
    let ys = code.corners.map(|p| p[1] * image.height() as f64);
    let min_x = xs.into_iter().fold(f64::INFINITY, f64::min);
    let max_x = xs.into_iter().fold(0.0, f64::max);
    let min_y = ys.into_iter().fold(f64::INFINITY, f64::min);
    let max_y = ys.into_iter().fold(0.0, f64::max);
    // Preserve a quiet zone around the selected code so its crop can be reused.
    let margin = ((max_x - min_x).max(max_y - min_y) * 0.2).ceil();
    let x = (min_x - margin).floor().max(0.0) as u32;
    let y = (min_y - margin).floor().max(0.0) as u32;
    let right = (max_x + margin).ceil().min(image.width() as f64) as u32;
    let bottom = (max_y + margin).ceil().min(image.height() as f64) as u32;
    let (width, height) = (right.saturating_sub(x), bottom.saturating_sub(y));
    if width == 0 || height == 0 {
        return Err("The QR image could not be read.".into());
    }
    let mut png = std::io::Cursor::new(Vec::new());
    image
        .crop_imm(x, y, width, height)
        .write_to(&mut png, image::ImageFormat::Png)
        .map_err(|_| "Could not save this QR code.".to_string())?;
    Ok((png.into_inner(), width, height))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn links_are_explicit_web_urls_without_credentials_or_controls() {
        for text in [
            "javascript:alert(1)",
            "file:///tmp/a",
            "data:text/html,hi",
            "https://trusted.example@evil.example",
            "https://a.example\\@b.example",
            "https://a.example\n",
            " https://a.example",
            "example.com",
            "https://",
        ] {
            assert!(link(text).is_none(), "{text}");
        }
        assert_eq!(
            link("https://example.com/a?q=1").unwrap().host_str(),
            Some("example.com")
        );
        assert!(describe(0, [[0.0; 2]; 4], Some("https://xn--pple-43d.com".into())).suspicious);
        assert!(describe(0, [[0.0; 2]; 4], Some("http://127.0.0.1".into())).suspicious);
        assert!(describe(0, [[0.0; 2]; 4], Some("https://[::1]".into())).suspicious);
    }
    #[test]
    fn invalid_images_fail_without_results() {
        assert!(decode(b"not a PNG").is_err());
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_luma8(200, 100)
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        assert!(decode(png.get_ref()).unwrap().2.is_empty());
    }

    #[test]
    fn real_single_multi_duplicate_and_unicode_codes_keep_their_positions() {
        let single = decode(include_bytes!("../tests/fixtures/qr/url.png"))
            .unwrap()
            .2;
        assert_eq!(single.len(), 1);
        assert_eq!(
            single[0].text.as_deref(),
            Some("https://example.org/kiri-safe")
        );
        let multi = decode(include_bytes!("../tests/fixtures/qr/multi.png"))
            .unwrap()
            .2;
        assert_eq!(multi.len(), 3);
        assert_eq!(multi.iter().filter(|c| c.text == single[0].text).count(), 2);
        assert!(multi
            .iter()
            .any(|c| c.text.as_deref() == Some("Kiri QR 测试")));
        assert!(multi.iter().all(|c| c
            .corners
            .iter()
            .all(|p| p.iter().all(|v| *v >= 0.0 && *v <= 1.0))));
        assert!(multi.iter().any(|c| c.corners[0][0] > 0.6));
        for code in multi {
            let (crop, _, _) =
                crop_code(include_bytes!("../tests/fixtures/qr/multi.png"), &code).unwrap();
            assert!(decode(&crop).unwrap().2.iter().any(|c| c.text == code.text));
        }
    }

    #[test]
    fn damaged_codes_stay_locatable_and_unsafe_payloads_stay_plain_content() {
        let broken = decode(include_bytes!("../tests/fixtures/qr/damaged.png"))
            .unwrap()
            .2;
        assert!(!broken.is_empty());
        assert!(broken.iter().all(|c| c.text.is_none() && c.url.is_none()));
        let unsafe_code = decode(include_bytes!("../tests/fixtures/qr/unsafe.png"))
            .unwrap()
            .2;
        assert_eq!(unsafe_code.len(), 1);
        assert_eq!(unsafe_code[0].text.as_deref(), Some("javascript:alert(1)"));
        assert!(unsafe_code[0].url.is_none());
        assert!(unsafe_code[0].suspicious);
    }
}
