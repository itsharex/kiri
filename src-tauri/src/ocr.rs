//! Local OCR — Vision on macOS, Windows.Media.Ocr on Windows, Tesseract on Linux.
//! Uses accurate recognition with language correction and automatic language
//! detection, returning the top candidate per line.

use anyhow::{anyhow, Result};

#[cfg(windows)]
fn rgba_to_bgra_in_place(bytes: &mut [u8]) {
    for pixel in bytes.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
}

#[cfg(target_os = "macos")]
fn make_macos_text_request() -> objc2::rc::Retained<objc2_vision::VNRecognizeTextRequest> {
    use objc2_vision::{VNRecognizeTextRequest, VNRequestTextRecognitionLevel};

    let request = VNRecognizeTextRequest::new();
    request.setRecognitionLevel(VNRequestTextRecognitionLevel::Accurate);
    request.setUsesLanguageCorrection(true);
    request.setAutomaticallyDetectsLanguage(true);
    request
}

#[cfg(target_os = "macos")]
pub fn recognize_text(png: &[u8]) -> Result<String> {
    use objc2::rc::Retained;
    use objc2::AnyThread;
    use objc2_foundation::{NSArray, NSData, NSDictionary};
    use objc2_vision::{VNImageRequestHandler, VNRecognizedTextObservation};

    let data = NSData::with_bytes(png);
    let handler = VNImageRequestHandler::initWithData_options(
        VNImageRequestHandler::alloc(),
        &data,
        &NSDictionary::new(),
    );

    let request = make_macos_text_request();

    let request_ref: &objc2_vision::VNRequest = &request;
    let requests: Retained<NSArray<objc2_vision::VNRequest>> = NSArray::from_slice(&[request_ref]);
    let result = handler.performRequests_error(&requests);
    if result.is_err() {
        return Err(anyhow!("Text Recognition Failed"));
    }

    let mut lines = Vec::new();
    if let Some(observations) = request.results() {
        for observation in observations.iter() {
            if let Ok(recognized) = observation.downcast::<VNRecognizedTextObservation>() {
                let candidates = recognized.topCandidates(1);
                if let Some(top) = candidates.firstObject() {
                    lines.push(top.string().to_string());
                }
            }
        }
    }
    let text = lines.join("\n");
    // A successful recognition with no characters is a normal empty result.
    Ok(text)
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::make_macos_text_request;

    #[test]
    fn blank_image_is_a_successful_empty_local_result() {
        let image = image::RgbaImage::from_pixel(320, 180, image::Rgba([255, 255, 255, 255]));
        let mut png = std::io::Cursor::new(Vec::new());
        image.write_to(&mut png, image::ImageFormat::Png).unwrap();
        assert!(super::recognize_text(&png.into_inner()).unwrap().trim().is_empty());
    }

    #[test]
    fn macos_request_uses_automatic_language_detection() {
        let request = make_macos_text_request();
        assert!(request.automaticallyDetectsLanguage());
    }
}

#[cfg(windows)]
pub fn recognize_text(png: &[u8]) -> Result<String> {
    use windows::Graphics::Imaging::BitmapPixelFormat;
    use windows::Media::Ocr::OcrEngine;
    use windows::Storage::Streams::DataWriter;

    let image = image::load_from_memory(png).map_err(|error| anyhow!("{error}"))?;
    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();
    let mut raw = rgba.into_raw();
    // Windows.Media.Ocr expects the same BGRA8 SoftwareBitmap shape used by
    // Microsoft's OCR samples. `image` decodes to RGBA8, so swap red and blue
    // before declaring the WinRT buffer as BGRA8. Declaring RGBA8 here causes
    // RecognizeAsync to reject otherwise valid screenshots on Windows 11.
    rgba_to_bgra_in_place(&mut raw);

    // Write the pixel bytes into an IBuffer via DataWriter (the windows crate
    // no longer exposes Buffer::as_mut for raw writes).
    let writer = DataWriter::new().map_err(|error| anyhow!("{error}"))?;
    writer
        .WriteBytes(&raw)
        .map_err(|error| anyhow!("{error}"))?;
    let buffer = writer.DetachBuffer().map_err(|error| anyhow!("{error}"))?;

    let bitmap: windows::Graphics::Imaging::SoftwareBitmap =
        windows::Graphics::Imaging::SoftwareBitmap::CreateCopyFromBuffer(
            &buffer,
            BitmapPixelFormat::Bgra8,
            width as i32,
            height as i32,
        )
        .map_err(|error| anyhow!("{error}"))?;

    let engine =
        OcrEngine::TryCreateFromUserProfileLanguages().map_err(|error| anyhow!("{error}"))?;
    let operation = engine
        .RecognizeAsync(&bitmap)
        .map_err(|error| anyhow!("{error}"))?;
    let result = operation.join().map_err(|error| anyhow!("{error}"))?;

    let mut lines = Vec::new();
    let ocr_lines = result.Lines().map_err(|error| anyhow!("{error}"))?;
    for line in ocr_lines {
        lines.push(line.Text().map_err(|error| anyhow!("{error}"))?.to_string());
    }
    let text = lines.join("\n");
    // A successful recognition with no characters is a normal empty result.
    Ok(text)
}

#[cfg(all(test, windows))]
mod windows_tests {
    use super::rgba_to_bgra_in_place;

    #[test]
    fn converts_decoded_rgba_pixels_to_windows_bgra_order() {
        let mut pixels = vec![1, 2, 3, 4, 10, 20, 30, 40];
        rgba_to_bgra_in_place(&mut pixels);
        assert_eq!(pixels, [3, 2, 1, 4, 30, 20, 10, 40]);
    }
}

#[cfg(any(target_os = "linux", test))]
fn linux_ocr_languages(directory: &std::path::Path) -> Option<String> {
    let installed: Vec<_> = ["eng", "chi_sim", "jpn"]
        .into_iter()
        .filter(|language| directory.join(format!("{language}.traineddata")).is_file())
        .collect();
    (!installed.is_empty()).then(|| installed.join("+"))
}

#[cfg(any(target_os = "linux", test))]
fn linux_ocr_data(
    explicit_directory: Option<std::ffi::OsString>,
) -> Option<(std::path::PathBuf, String)> {
    let directories = if let Some(directory) = explicit_directory.filter(|value| !value.is_empty())
    {
        // Respect an explicit local installation without silently changing the
        // user's configured model directory or fetching missing data.
        vec![std::path::PathBuf::from(directory)]
    } else {
        [
            "/usr/share/tesseract-ocr/5/tessdata",
            "/usr/share/tesseract-ocr/4.00/tessdata",
            "/usr/share/tesseract-ocr/tessdata",
            "/usr/share/tessdata",
            "/usr/local/share/tessdata",
        ]
        .into_iter()
        .map(std::path::PathBuf::from)
        .collect()
    };
    directories.into_iter().find_map(|directory| {
        linux_ocr_languages(&directory).map(|languages| (directory, languages))
    })
}

#[cfg(target_os = "linux")]
pub fn recognize_text(png: &[u8]) -> Result<String> {
    let (directory, languages) = linux_ocr_data(std::env::var_os("TESSDATA_PREFIX"))
        .ok_or_else(|| anyhow!("Local OCR needs a Tesseract language pack. Install English, Simplified Chinese, or Japanese language data."))?;
    let data_path = directory
        .to_str()
        .ok_or_else(|| anyhow!("The installed Tesseract language data could not be loaded."))?;
    // Bind to the system library and already-installed models. Neither this
    // engine nor its error path launches a program or contacts a provider.
    let mut engine = tesseract::Tesseract::new(Some(data_path), Some(&languages))
        .map_err(|_| anyhow!("The installed Tesseract language data could not be loaded."))?
        .set_image_from_mem(png)
        .map_err(|_| anyhow!("Text Recognition Failed"))?;
    engine.set_page_seg_mode(tesseract::PageSegMode::PsmSparseText);
    let text = engine
        .get_text()
        .map_err(|_| anyhow!("Text Recognition Failed"))?;
    let text = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    Ok(text)
}

#[cfg(test)]
mod linux_model_tests {
    use super::{linux_ocr_data, linux_ocr_languages};

    #[test]
    fn local_ocr_only_selects_installed_supported_language_models() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("eng.traineddata"), []).unwrap();
        std::fs::write(directory.path().join("jpn.traineddata"), []).unwrap();
        std::fs::write(directory.path().join("osd.traineddata"), []).unwrap();
        assert_eq!(
            linux_ocr_languages(directory.path()).as_deref(),
            Some("eng+jpn")
        );
    }

    #[test]
    fn local_ocr_honors_the_explicit_data_directory() {
        let directory = tempfile::tempdir().unwrap();
        let explicit = directory.path().as_os_str().to_owned();
        assert!(linux_ocr_data(Some(explicit.clone())).is_none());
        std::fs::write(directory.path().join("chi_sim.traineddata"), []).unwrap();
        assert_eq!(
            linux_ocr_data(Some(explicit)),
            Some((directory.path().to_owned(), "chi_sim".into()))
        );
    }

    #[test]
    fn local_ocr_does_not_mistake_a_model_directory_for_a_model_file() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::create_dir(directory.path().join("eng.traineddata")).unwrap();
        assert!(linux_ocr_languages(directory.path()).is_none());
    }
}

#[cfg(all(test, target_os = "linux"))]
mod linux_recognition_tests {
    #[test]
    fn system_tesseract_recognizes_the_local_png_fixture() {
        // Fixed black text on white, generated solely for this test. Exercise
        // PNG decoding, the linked engine, installed models and returned text.
        let png = include_bytes!("../tests/fixtures/linux-ocr.png");
        let text = super::recognize_text(png)
            .expect("Linux OCR requires installed Tesseract language data");
        let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(
            normalized.contains("KIRI LINUX 123"),
            "unexpected OCR output: {normalized:?}"
        );
    }
}
