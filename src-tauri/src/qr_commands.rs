use crate::qr_controller::Scan;
use crate::{
    commands::{asset_dto, AssetDto, RectDto},
    core::asset::CaptureKind,
    qr::{self, QrCode},
    state::{emit_library_changed, AppState},
};
use base64::Engine;
use serde::Serialize;
use std::{io::Read, sync::Arc};
use tauri::{AppHandle, Manager, WebviewWindow};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanDto {
    request_id: String,
    image_url: String,
    width: u32,
    height: u32,
    codes: Vec<QrCode>,
}

#[tauri::command]
pub async fn scan_qr(
    window: WebviewWindow,
    app: AppHandle,
    request_id: String,
    selection: Option<RectDto>,
    asset_id: Option<String>,
) -> Result<ScanDto, String> {
    let id = uuid::Uuid::parse_str(&request_id).map_err(|_| "Invalid QR request.")?;
    let label = window.label().to_string();
    let image_owner = asset_id
        .as_deref()
        .map(uuid::Uuid::parse_str)
        .transpose()
        .map_err(|_| "Invalid image.")?;
    let owner = if image_owner
        .is_some_and(|id| crate::qr_controller::image_owner_matches(&label, id))
        && selection.is_none()
    {
        None
    } else if asset_id.is_none() && selection.is_some() {
        Some(crate::ocr_commands::require_active_overlay(&window, &app)?)
    } else {
        return Err("QR recognition is unavailable from this window.".into());
    };
    let state = app.state::<AppState>();
    let requests = state.qr_requests.clone();
    let identity = {
        let context = state.library.lock().unwrap();
        (
            context.expected_library_id(),
            context.expected_library_generation(),
        )
    };
    let permit = requests.begin(
        label.clone(),
        Scan {
            id,
            capture_id: owner.as_ref().map(|o| o.capture_id),
            asset_id: image_owner,
            library_identity: identity,
            png: Arc::from([]),
            codes: vec![],
        },
    )?;
    let result = tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        let png = if let Some(selection) = selection {
            let source = {
                let state = app.state::<AppState>();
                let capture = state.capture.lock().unwrap();
                let session = capture
                    .session
                    .as_ref()
                    .filter(|s| {
                        Some(s.capture_id) == owner.as_ref().map(|o| o.capture_id)
                            && s.overlay_labels.contains(&label)
                    })
                    .ok_or("The QR request was canceled.")?;
                crate::ocr_commands::FrozenPngSource {
                    png: session.display.png_data.clone(),
                    declared_width: session.display.pixel_width,
                    declared_height: session.display.pixel_height,
                    display_width: session.display.screen_frame.width,
                    display_height: session.display.screen_frame.height,
                    scale: session.display.backing_scale,
                }
            };
            crate::ocr_commands::crop_frozen_png(source, selection)?.png
        } else {
            let id = uuid::Uuid::parse_str(asset_id.as_deref().ok_or("Invalid image.")?)
                .map_err(|_| "Invalid image.")?;
            let state = app.state::<AppState>();
            let mut context = state.library.lock().unwrap();
            if (
                context.expected_library_id(),
                context.expected_library_generation(),
            ) != identity
            {
                return Err("The library changed during QR recognition.".into());
            }
            let library = context.library().map_err(|e| e.to_string())?;
            let asset = library
                .asset_by_id(&id)
                .filter(|a| a.kind == CaptureKind::Image && a.trashed_at.is_none())
                .ok_or("Choose a screenshot from the library.")?;
            let path = library
                .readable_asset_url(asset)
                .map_err(|e| e.to_string())?;
            let mut png = Vec::new();
            std::fs::File::open(path)
                .map_err(|_| "The QR image could not be read.")?
                .take(qr::MAX_PNG_BYTES as u64 + 1)
                .read_to_end(&mut png)
                .map_err(|_| "The QR image could not be read.")?;
            png
        };
        let (width, height, codes) = qr::decode(&png)?;
        let image_url = format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(&png)
        );
        let state = app.state::<AppState>();
        // Capture teardown and cancellation must not publish a stale result.
        let capture = state.capture.lock().unwrap();
        if let Some(owner) = &owner {
            if !capture.session.as_ref().is_some_and(|s| {
                s.capture_id == owner.capture_id && s.overlay_labels.contains(&label)
            }) {
                requests.clear(&label);
                return Err("The QR request was canceled.".into());
            }
        }
        let context = state.library.lock().unwrap();
        if (
            context.expected_library_id(),
            context.expected_library_generation(),
        ) != identity
        {
            requests.cancel(&label, id);
            return Err("The library changed during QR recognition.".into());
        }
        requests.publish(&label, id, Arc::from(png), codes.clone())?;
        Ok(ScanDto {
            request_id,
            image_url,
            width,
            height,
            codes,
        })
    })
    .await
    .map_err(|_| "QR recognition failed.".to_string())?;
    result
}

#[tauri::command]
pub fn cancel_qr(window: WebviewWindow, app: AppHandle, request_id: String) {
    let state = app.state::<AppState>();
    if let Ok(id) = uuid::Uuid::parse_str(&request_id) {
        state.qr_requests.cancel(window.label(), id);
    }
}

fn selected(
    window: &WebviewWindow,
    app: &AppHandle,
    request_id: &str,
    index: usize,
) -> Result<(Scan, QrCode), String> {
    let state = app.state::<AppState>();
    let capture = state.capture.lock().unwrap();
    let scans = state.qr_requests.scans.lock().unwrap();
    let scan = scans
        .get(window.label())
        .filter(|s| s.id.to_string() == request_id)
        .ok_or("The QR request was canceled.")?;
    if let Some(id) = scan.capture_id {
        if !capture.session.as_ref().is_some_and(|s| {
            s.capture_id == id && s.overlay_labels.iter().any(|l| l == window.label())
        }) {
            return Err("The QR request was canceled.".into());
        }
    } else if !scan
        .asset_id
        .is_some_and(|id| crate::qr_controller::image_owner_matches(window.label(), id))
    {
        return Err("QR recognition is unavailable from this window.".into());
    }
    let context = state.library.lock().unwrap();
    if (
        context.expected_library_id(),
        context.expected_library_generation(),
    ) != scan.library_identity
    {
        return Err("The library changed during QR recognition.".into());
    }
    let code = scan
        .codes
        .get(index)
        .filter(|c| c.text.is_some())
        .ok_or("This QR code could not be decoded.")?
        .clone();
    Ok((scan.clone(), code))
}

#[tauri::command]
pub async fn qr_action(
    window: WebviewWindow,
    app: AppHandle,
    request_id: String,
    index: usize,
    action: String,
) -> Result<Option<AssetDto>, String> {
    let (scan, code) = selected(&window, &app, &request_id, index)?;
    let text = code
        .text
        .as_deref()
        .ok_or("This QR code could not be decoded.")?;
    match action.as_str() {
        "copy" => {
            crate::platform::write_text_to_clipboard(text).map_err(|e| e.to_string())?;
            Ok(None)
        }
        "open" => {
            open_link(&app, text).await?;
            Ok(None)
        }
        "favorite" | "unfavorite" => {
            let label = window.label().to_string();
            tauri::async_runtime::spawn_blocking(move || {
                let crop = if action == "favorite" {
                    Some(qr::crop_code(&scan.png, &code)?)
                } else {
                    None
                };
                let state = app.state::<AppState>();
                let capture = state.capture.lock().unwrap();
                let scans = state.qr_requests.scans.lock().unwrap();
                if !scans.get(&label).is_some_and(|s| s.id == scan.id)
                    || scan.capture_id.is_some_and(|id| {
                        !capture.session.as_ref().is_some_and(|s| {
                            s.capture_id == id && s.overlay_labels.contains(&label)
                        })
                    })
                {
                    return Err("The QR request was canceled.".into());
                }
                let mut context = state.library.lock().unwrap();
                if (
                    context.expected_library_id(),
                    context.expected_library_generation(),
                ) != scan.library_identity
                {
                    return Err("The library changed during QR recognition.".into());
                }
                let library = context.library_mut().map_err(|e| e.to_string())?;
                let asset = if let Some((png, width, height)) = crop {
                    Some(
                        library
                            .import_qr(&png, width as i64, height as i64, code.text.unwrap())
                            .map_err(|e| e.to_string())?,
                    )
                } else {
                    if let Some(asset) = library
                        .all_assets(false)
                        .iter()
                        .find(|a| a.qr_text == code.text)
                    {
                        library
                            .move_to_trash(&asset.id)
                            .map_err(|e| e.to_string())?;
                    }
                    None
                };
                drop(context);
                drop(scans);
                drop(capture);
                emit_library_changed(&app);
                Ok(asset.as_ref().map(asset_dto))
            })
            .await
            .map_err(|_| "Could not save this QR code.".to_string())?
        }
        _ => Err("Invalid QR action.".into()),
    }
}

#[tauri::command]
pub fn list_qr_favorites(
    window: WebviewWindow,
    app: AppHandle,
    query: String,
) -> Result<Vec<AssetDto>, String> {
    if window.label() != "library" {
        return Err("QR favorites are only available from the library.".into());
    }
    let state = app.state::<AppState>();
    let mut context = state.library.lock().unwrap();
    Ok(context
        .library()
        .map_err(|e| e.to_string())?
        .search(&query, false)
        .iter()
        .filter(|a| a.qr_text.is_some() && a.is_favorite)
        .map(asset_dto)
        .collect())
}

#[tauri::command]
pub async fn qr_favorite_action(
    window: WebviewWindow,
    app: AppHandle,
    id: String,
    action: String,
) -> Result<(), String> {
    if window.label() != "library" {
        return Err("QR favorites are only available from the library.".into());
    }
    let id = uuid::Uuid::parse_str(&id).map_err(|_| "Invalid QR favorite.")?;
    let text = {
        let state = app.state::<AppState>();
        let mut context = state.library.lock().unwrap();
        let library = context.library_mut().map_err(|e| e.to_string())?;
        let asset = library
            .asset_by_id(&id)
            .filter(|a| a.trashed_at.is_none() && a.is_favorite && a.qr_text.is_some())
            .ok_or("Invalid QR favorite.")?;
        let text = asset.qr_text.clone().unwrap();
        if action == "remove" {
            library.move_to_trash(&id).map_err(|e| e.to_string())?;
        }
        text
    };
    match action.as_str() {
        "copy" => crate::platform::write_text_to_clipboard(&text).map_err(|e| e.to_string()),
        "copyImage" => crate::commands::copy_asset(app, window, id.to_string()),
        "open" => open_link(&app, &text).await,
        "remove" => {
            emit_library_changed(&app);
            Ok(())
        }
        _ => Err("Invalid QR action.".into()),
    }
}

async fn open_link(app: &AppHandle, text: &str) -> Result<(), String> {
    let url = qr::link(text)
        .ok_or("Only explicit HTTP or HTTPS links without credentials can be opened.")?
        .to_string();
    let (tx, rx) = std::sync::mpsc::channel();
    app.run_on_main_thread(move || {
        let _ = tx.send(open_web_url(&url));
    })
    .map_err(|_| "Could not open this link.")?;
    tauri::async_runtime::spawn_blocking(move || {
        rx.recv()
            .map_err(|_| "Could not open this link.".to_string())?
    })
    .await
    .map_err(|_| "Could not open this link.".to_string())?
}

fn open_web_url(url: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let url =
            objc2_foundation::NSURL::URLWithString(&objc2_foundation::NSString::from_str(url))
                .ok_or("Could not open this link.")?;
        if !objc2_app_kit::NSWorkspace::sharedWorkspace().openURL(&url) {
            return Err("Could not open this link.".into());
        }
    }
    #[cfg(windows)]
    {
        use windows::{
            core::PCWSTR,
            Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL},
        };
        let url: Vec<u16> = url.encode_utf16().chain(Some(0)).collect();
        let verb: Vec<u16> = "open".encode_utf16().chain(Some(0)).collect();
        let result = unsafe {
            ShellExecuteW(
                None,
                PCWSTR(verb.as_ptr()),
                PCWSTR(url.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            )
        };
        if result.0 as isize <= 32 {
            return Err("Could not open this link.".into());
        }
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map_err(|_| "Could not open this link.")?;
    }
    Ok(())
}
