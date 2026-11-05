//! Deeboo MIDI Sync: tray app shell, MIDI bridge, settings and the state event.

mod bridge;
mod clock;
mod commands;
mod settings;
mod tray;

use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

use bridge::Bridge;
use clock_core::Command;
use commands::{SettingsStore, StateDto};
use tauri::image::Image;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{App, AppHandle, Emitter, Manager, WindowEvent};
use tauri_plugin_positioner::{Position, WindowExt};
use tray::TrayFrame;

const POPOVER: &str = "popover";
const TRAY_ID: &str = "main";
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

/// Menu bar state owned by the emitter loop, so the tray is touched only on change.
struct TrayState {
    epoch: Instant,
    last_beats: u64,
    last_beat_change_us: u64,
    frame: Option<TrayFrame>,
    title: Option<String>,
}

impl TrayState {
    fn new() -> Self {
        Self {
            epoch: Instant::now(),
            last_beats: 0,
            last_beat_change_us: 0,
            frame: None,
            title: None,
        }
    }
}

fn tray_image(frame: TrayFrame) -> Option<Image<'static>> {
    let bytes: &'static [u8] = match frame {
        TrayFrame::GreenOn => include_bytes!("../icons/tray/vinyl-green-on@2x.png"),
        TrayFrame::GreenDim => include_bytes!("../icons/tray/vinyl-green-dim@2x.png"),
        TrayFrame::RedOn => include_bytes!("../icons/tray/vinyl-red-on@2x.png"),
        TrayFrame::RedDim => include_bytes!("../icons/tray/vinyl-red-dim@2x.png"),
    };
    Image::from_bytes(bytes).ok()
}

fn update_tray(app: &AppHandle, dto: &StateDto, state: &mut TrayState) {
    let now_us = u64::try_from(state.epoch.elapsed().as_micros()).unwrap_or(u64::MAX);
    if dto.input_beats != state.last_beats {
        state.last_beats = dto.input_beats;
        state.last_beat_change_us = now_us;
    }
    let frame = tray::tray_frame(dto.in_ok, dto.out_ok, state.last_beat_change_us, now_us);
    let title = tray::tray_title(dto.bpm);
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    if state.frame != Some(frame)
        && let Some(image) = tray_image(frame)
    {
        if let Err(e) = tray.set_icon(Some(image)) {
            log::warn!("cannot set tray icon: {e}");
        } else {
            state.frame = Some(frame);
        }
    }
    if state.title.as_deref() != Some(title.as_str()) {
        if let Err(e) = tray.set_title(Some(title.as_str())) {
            log::warn!("cannot set tray title: {e}");
        } else {
            state.title = Some(title);
        }
    }
}

/// One emitter tick: publish the state to the popover and the menu bar.
fn emit_tick(app: &AppHandle, bridge: &Bridge, tray_state: &mut TrayState) {
    let dto = StateDto::from(&bridge.snapshot());
    if let Err(e) = app.emit(STATE_EVENT, &dto) {
        log::warn!("cannot emit state: {e}");
    }
    update_tray(app, &dto, tray_state);
}

fn spawn_emitter(app: AppHandle, bridge: Bridge) {
    thread::spawn(move || {
        let mut tray_state = TrayState::new();
        loop {
            emit_tick(&app, &bridge, &mut tray_state);
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

    let icon = tray_image(TrayFrame::RedOn).ok_or("tray icon is not a valid PNG")?;
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .icon_as_template(false)
        .title(tray::tray_title(None))
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
