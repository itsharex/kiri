//! Offline QR recognition. Coordinates remain attached to each physical code,
//! including repeated payloads and codes that can be located but not decoded.
use image::ImageDecoder;
use serde::Serialize;

pub const MAX_PNG_BYTES: usize = 20 * 1024 * 1024;
const MAX_PIXELS: u64 = 32_000_000;
const MAX_CODES: usize = 64;

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
    let mut codes = identify(&mut decoder, width, height, &gray);
    // A whole-image Otsu threshold can erase a locally low-contrast code when
    // unrelated bright/dark content dominates the image. Merge all three
    // bounded contrast passes, even after a partial result; different physical
    // codes may require different thresholds. Coordinates stay unchanged.
    let mut binary = vec![0; gray.len()];
    for threshold in [64, 128, 192] {
        for (pixel, value) in binary.iter_mut().zip(gray.iter()) {
            *pixel = if *value < threshold { 0 } else { 255 };
        }
        merge_codes(
            &mut codes,
            identify(&mut decoder, width, height, &binary),
            width,
            height,
        );
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

fn identify(decoder: &mut quircs::Quirc, width: u32, height: u32, gray: &[u8]) -> Vec<QrCode> {
    let mut codes = Vec::new();
    for code in decoder
        .identify(width as usize, height as usize, gray)
        .take(MAX_CODES)
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
    codes
}

fn same_code_position(a: &QrCode, b: &QrCode, width: u32, height: u32) -> bool {
    let geometry = |code: &QrCode| {
        let points = code
            .corners
            .map(|p| [p[0] * width as f64, p[1] * height as f64]);
        let center = [
            points.iter().map(|p| p[0]).sum::<f64>() / 4.0,
            points.iter().map(|p| p[1]).sum::<f64>() / 4.0,
        ];
        let edges = std::array::from_fn::<_, 4, _>(|i| {
            let next = points[(i + 1) % 4];
            (points[i][0] - next[0]).hypot(points[i][1] - next[1])
        });
        let span = edges.iter().sum::<f64>() / 4.0;
        let shortest = edges.into_iter().fold(f64::INFINITY, f64::min);
        (center, span, shortest)
    };
    let (a_center, a_span, a_shortest) = geometry(a);
    let (b_center, b_span, b_shortest) = geometry(b);
    let smaller = a_span.min(b_span);
    // Use the short edge for positional tolerance so adjacent stretched codes
    // do not merge merely because their long edges are close together.
    let tolerance = a_shortest.min(b_shortest) * 0.15;
    smaller > 0.0
        && smaller >= a_span.max(b_span) * 0.75
        && (a_center[0] - b_center[0]).hypot(a_center[1] - b_center[1]) <= tolerance
}

fn merge_codes(codes: &mut Vec<QrCode>, candidates: Vec<QrCode>, width: u32, height: u32) {
    for candidate in candidates {
        if let Some(existing) = codes
            .iter_mut()
            .find(|code| same_code_position(code, &candidate, width, height))
        {
            // A later pass can decode a previously located code. Keep the first
            // readable result if another pass fails or disagrees.
            if existing.text.is_none() && candidate.text.is_some() {
                *existing = candidate;
            }
        } else if codes.len() < MAX_CODES {
            // Deduplicate physical positions, never payloads: repeated content
            // at different positions still needs separate clickable targets.
            codes.push(candidate);
        }
    }
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

    #[test]
    fn low_contrast_codes_survive_unrelated_bright_and_dark_backgrounds() {
        let source = image::load_from_memory(include_bytes!("../tests/fixtures/qr/url.png"))
            .unwrap()
            .into_luma8();
        for (dark, light, background) in [(0, 100, 255), (100, 160, 0), (160, 230, 0)] {
            let mut code = source.clone();
            for value in code.iter_mut() {
                *value = if *value < 128 { dark } else { light };
            }
            let mut canvas = image::GrayImage::from_pixel(960, 480, image::Luma([background]));
            image::imageops::replace(&mut canvas, &code, 32, 32);
            image::imageops::replace(&mut canvas, &code, 600, 120);
            // The old single whole-image threshold loses both physical codes.
            assert!(identify(&mut quircs::Quirc::default(), 960, 480, &canvas).is_empty());
            let mut png = std::io::Cursor::new(Vec::new());
            canvas.write_to(&mut png, image::ImageFormat::Png).unwrap();
            let (width, height, codes) = decode(png.get_ref()).unwrap();
            assert_eq!((width, height), (960, 480));
            assert_eq!(codes.len(), 2);
            for (index, code) in codes.iter().enumerate() {
                assert_eq!(code.index, index);
                assert_eq!(code.text.as_deref(), Some("https://example.org/kiri-safe"));
            }
            assert!((codes[0].corners[0][0] * 960.0 - 64.0).abs() <= 1.0);
            assert!((codes[1].corners[0][0] * 960.0 - 632.0).abs() <= 1.0);
            assert!((codes[1].corners[0][1] * 480.0 - 152.0).abs() <= 1.0);
        }
    }

    #[test]
    fn empty_threshold_retries_do_not_invent_codes() {
        for value in [0, 64, 128, 192, 255] {
            let blank = image::GrayImage::from_pixel(320, 240, image::Luma([value]));
            let mut png = std::io::Cursor::new(Vec::new());
            blank.write_to(&mut png, image::ImageFormat::Png).unwrap();
            assert!(decode(png.get_ref()).unwrap().2.is_empty());
        }
    }

    #[test]
    fn partial_scans_merge_codes_from_different_contrast_ranges() {
        let url = image::load_from_memory(include_bytes!("../tests/fixtures/qr/url.png"))
            .unwrap()
            .into_luma8();
        let text = image::load_from_memory(include_bytes!("../tests/fixtures/qr/text.png"))
            .unwrap()
            .into_luma8();
        let mut canvas = image::GrayImage::from_pixel(960, 480, image::Luma([255]));
        for (source, dark, light, x, y) in [
            (&url, 0, 255, 32, 32),
            (&text, 0, 100, 352, 140),
            (&url, 150, 255, 672, 32),
        ] {
            let mut code = source.clone();
            for value in code.iter_mut() {
                *value = if *value < 128 { dark } else { light };
            }
            image::imageops::replace(&mut canvas, &code, x, y);
        }
        let initial = identify(&mut quircs::Quirc::default(), 960, 480, &canvas);
        assert_eq!(initial.len(), 2);
        assert!(initial
            .iter()
            .all(|code| code.text.as_deref() == Some("https://example.org/kiri-safe")));
        let mut png = std::io::Cursor::new(Vec::new());
        canvas.write_to(&mut png, image::ImageFormat::Png).unwrap();
        let (_, _, codes) = decode(png.get_ref()).unwrap();
        assert_eq!(codes.len(), 3);
        assert_eq!(
            codes
                .iter()
                .filter(|code| code.text.as_deref() == Some("https://example.org/kiri-safe"))
                .count(),
            2
        );
        assert_eq!(codes[2].text.as_deref(), Some("Kiri QR 测试"));
        for (index, code) in codes.iter().enumerate() {
            assert_eq!(code.index, index);
        }
        assert!((codes[0].corners[0][0] * 960.0 - 64.0).abs() <= 1.0);
        assert!((codes[1].corners[0][0] * 960.0 - 704.0).abs() <= 1.0);
        assert!((codes[2].corners[0][0] * 960.0 - 384.0).abs() <= 1.0);
    }

    #[test]
    fn initially_empty_scans_keep_results_from_later_thresholds() {
        let mut canvas = image::GrayImage::from_pixel(1280, 720, image::Luma([255]));
        for (png, dark, light, x, y) in [
            (
                include_bytes!("../tests/fixtures/qr/text.png").as_slice(),
                0,
                100,
                128,
                128,
            ),
            (
                include_bytes!("../tests/fixtures/qr/url.png").as_slice(),
                100,
                160,
                672,
                224,
            ),
        ] {
            let mut code = image::load_from_memory(png).unwrap().into_luma8();
            for value in code.iter_mut() {
                *value = if *value < 128 { dark } else { light };
            }
            image::imageops::replace(&mut canvas, &code, x, y);
        }
        assert!(identify(&mut quircs::Quirc::default(), 1280, 720, &canvas).is_empty());
        let mut png = std::io::Cursor::new(Vec::new());
        canvas.write_to(&mut png, image::ImageFormat::Png).unwrap();
        let (_, _, codes) = decode(png.get_ref()).unwrap();
        assert_eq!(codes.len(), 2);
        assert_eq!(codes[0].text.as_deref(), Some("Kiri QR 测试"));
        assert_eq!(
            codes[1].text.as_deref(),
            Some("https://example.org/kiri-safe")
        );
    }

    #[test]
    fn merging_upgrades_unreadable_codes_without_duplicating_or_downgrading_them() {
        let corners = [[0.1, 0.1], [0.2, 0.1], [0.2, 0.3], [0.1, 0.3]];
        let mut codes = vec![describe(0, corners, None)];
        let mut shifted = corners.map(|p| [p[0] + 0.002, p[1] + 0.004]);
        shifted.rotate_left(1);
        let readable = describe(0, shifted, Some("https://example.org/decoded".into()));
        merge_codes(&mut codes, vec![readable.clone()], 1000, 500);
        assert_eq!(codes.len(), 1);
        assert_eq!(codes[0].text, readable.text);
        assert_eq!(codes[0].url, readable.url);
        merge_codes(
            &mut codes,
            vec![
                describe(0, corners, None),
                describe(0, corners, Some("conflicting".into())),
            ],
            1000,
            500,
        );
        assert_eq!(codes.len(), 1);
        assert_eq!(codes[0].text, readable.text);
        let neighbor = describe(
            0,
            corners.map(|p| [p[0] + 0.11, p[1]]),
            readable.text.clone(),
        );
        merge_codes(&mut codes, vec![neighbor], 1000, 500);
        assert_eq!(codes.len(), 2);
    }

    #[test]
    fn merging_caps_physical_codes_but_still_upgrades_existing_results() {
        let corners = [[0.001, 0.1], [0.005, 0.1], [0.005, 0.2], [0.001, 0.2]];
        let candidates = (0..80)
            .map(|index| {
                describe(
                    index,
                    corners.map(|p| [p[0] + index as f64 * 0.01, p[1]]),
                    None,
                )
            })
            .collect();
        let mut codes = Vec::new();
        merge_codes(&mut codes, candidates, 1000, 500);
        assert_eq!(codes.len(), MAX_CODES);
        merge_codes(
            &mut codes,
            vec![describe(0, corners, Some("decoded at capacity".into()))],
            1000,
            500,
        );
        assert_eq!(codes.len(), MAX_CODES);
        assert_eq!(codes[0].text.as_deref(), Some("decoded at capacity"));
    }

    #[test]
    fn adjacent_stretched_codes_keep_separate_positions() {
        let source = image::load_from_memory(include_bytes!("../tests/fixtures/qr/url.png"))
            .unwrap()
            .into_luma8();
        let stretched =
            image::imageops::resize(&source, 740, 37, image::imageops::FilterType::Nearest);
        let mut dark = stretched.clone();
        for value in dark.iter_mut() {
            *value = if *value < 128 { 0 } else { 100 };
        }
        let mut canvas = image::GrayImage::from_pixel(800, 120, image::Luma([255]));
        image::imageops::replace(&mut canvas, &stretched, 32, 32);
        image::imageops::replace(&mut canvas, &dark, 32, 69);
        let mut png = std::io::Cursor::new(Vec::new());
        canvas.write_to(&mut png, image::ImageFormat::Png).unwrap();
        let (_, _, codes) = decode(png.get_ref()).unwrap();
        assert_eq!(codes.len(), 2);
        assert!(codes
            .iter()
            .all(|code| code.text.as_deref() == Some("https://example.org/kiri-safe")));
        assert!(codes[1].corners[0][1] > codes[0].corners[0][1]);
    }
}
