//! Deeboo MIDI Sync: tray app shell, MIDI bridge, settings and the state event.

mod bridge;
mod clock;
mod commands;
mod settings;

use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

use bridge::Bridge;
use clock_core::Command;
use commands::{SettingsStore, StateDto};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{App, AppHandle, Emitter, Manager, WindowEvent};
use tauri_plugin_positioner::{Position, WindowExt};

const POPOVER: &str = "popover";
const STATE_EVENT: &str = "deeboo://state";
const EMIT_PERIOD: Duration = Duration::from_millis(33);
/// A tray click this soon after a blur hid the popover is the same click that caused the blur.
const BLUR_CLICK_WINDOW: Duration = Duration::from_millis(250);

/// When the popover was last hidden because it lost focus.
#[derive(Default)]
struct BlurHide(Mutex<Option<Instant>>);

fn hidden_by_blur_just_now(hidden_at: Option<Instant>, now: Instant) -> bool {
    hidden_at.is_some_and(|t| now.saturating_duration_since(t) < BLUR_CLICK_WINDOW)
}

fn toggle_popover(app: &AppHandle) {
    let Some(window) = app.get_webview_window(POPOVER) else {
        log::error!("popover window is missing");
        return;
    };
    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
        return;
    }
    let blur_hide = app.state::<BlurHide>();
    let hidden_at = *blur_hide
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if hidden_by_blur_just_now(hidden_at, Instant::now()) {
        return;
    }
    if let Err(e) = window.move_window(Position::TrayBottomCenter) {
        log::error!("cannot position popover: {e}");
    }
    let _ = window.show();
    let _ = window.set_focus();
}

fn on_popover_blur(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(POPOVER) {
        let _ = window.hide();
    }
    *app.state::<BlurHide>()
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Instant::now());
    app.state::<Bridge>().command(Command::NudgeStop);
}

/// One emitter tick: publish the state to the popover.
fn emit_tick(app: &AppHandle, bridge: &Bridge) {
    let dto = StateDto::from(&bridge.snapshot());
    if let Err(e) = app.emit(STATE_EVENT, &dto) {
        log::warn!("cannot emit state: {e}");
    }
}

fn spawn_emitter(app: AppHandle, bridge: Bridge) {
    thread::spawn(move || {
        loop {
            emit_tick(&app, &bridge);
            thread::sleep(EMIT_PERIOD);
        }
    });
}

fn setup(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "macos")]
    app.set_activation_policy(tauri::ActivationPolicy::Accessory);

    let path = app.path().app_config_dir()?.join("settings.json");
    let current = settings::load(&path);
    let bridge = Bridge::start(
        &current.input_name,
        &current.output_name,
        current.nudge_strength_pct,
    );
    app.manage(bridge.clone());
    app.manage(SettingsStore {
        path,
        current: Mutex::new(current),
    });
    app.manage(BlurHide::default());
    spawn_emitter(app.handle().clone(), bridge);

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
        let handle = app.handle().clone();
        window.on_window_event(move |event| {
            if let WindowEvent::Focused(false) = event {
                on_popover_blur(&handle);
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
        .invoke_handler(tauri::generate_handler![
            commands::play,
            commands::pause,
            commands::nudge_start,
            commands::nudge_stop,
            commands::get_settings,
            commands::save_settings,
            commands::quit,
        ])
        .setup(setup)
        .run(tauri::generate_context!())
        .expect("error while running Deeboo MIDI Sync");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn click_right_after_blur_hide_is_ignored() {
        let t0 = Instant::now();
        assert!(hidden_by_blur_just_now(
            Some(t0),
            t0 + Duration::from_millis(100)
        ));
        assert!(!hidden_by_blur_just_now(
            Some(t0),
            t0 + Duration::from_millis(300)
        ));
        assert!(!hidden_by_blur_just_now(None, t0));
    }
}
