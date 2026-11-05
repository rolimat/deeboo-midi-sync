//! Deeboo MIDI Sync: tray app shell and MIDI bridge. Commands arrive in a later task.

#[expect(
    dead_code,
    reason = "commands and state event (task 8) use the rest of the API"
)]
mod bridge;
mod clock;

use bridge::Bridge;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{App, AppHandle, Manager, WindowEvent};
use tauri_plugin_positioner::{Position, WindowExt};

const POPOVER: &str = "popover";
const DEFAULT_IN: &str = "Deeboo In";
const DEFAULT_OUT: &str = "Deeboo Out";
const DEFAULT_NUDGE_PCT: f64 = 4.0;

fn toggle_popover(app: &AppHandle) {
    let Some(window) = app.get_webview_window(POPOVER) else {
        log::error!("popover window is missing");
        return;
    };
    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
        return;
    }
    if let Err(e) = window.move_window(Position::TrayBottomCenter) {
        log::error!("cannot position popover: {e}");
    }
    let _ = window.show();
    let _ = window.set_focus();
}

fn setup(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "macos")]
    app.set_activation_policy(tauri::ActivationPolicy::Accessory);

    app.manage(Bridge::start(DEFAULT_IN, DEFAULT_OUT, DEFAULT_NUDGE_PCT));

    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or("default window icon is missing")?;
    TrayIconBuilder::with_id("main")
        .icon(icon)
        .title("—")
        .on_tray_icon_event(|tray, event| {
            tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_popover(tray.app_handle());
            }
        })
        .build(app)?;

    if let Some(window) = app.get_webview_window(POPOVER) {
        let w = window.clone();
        window.on_window_event(move |event| {
            if let WindowEvent::Focused(false) = event {
                let _ = w.hide();
            }
        });
    }
    Ok(())
}

/// Starts the tray app.
///
/// # Panics
/// Panics if the Tauri runtime fails to start.
pub fn run() {
    env_logger::init();
    tauri::Builder::default()
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(setup)
        .run(tauri::generate_context!())
        .expect("error while running Deeboo MIDI Sync");
}
