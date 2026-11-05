//! Tauri commands and the state payload sent to the popover.
#![expect(
    clippy::needless_pass_by_value,
    reason = "Tauri injects State, AppHandle and arguments by value"
)]

use std::path::PathBuf;
use std::sync::Mutex;

use clock_core::{Command, Direction};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};
use tauri_plugin_autostart::ManagerExt;

use crate::bridge::{Bridge, BridgeSnapshot};
use crate::settings::{self, Settings};

/// Settings file location plus the settings currently in effect.
pub struct SettingsStore {
    pub path: PathBuf,
    pub current: Mutex<Settings>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StateDto {
    pub bpm: Option<f64>,
    pub beat: Option<u8>,
    pub playing: bool,
    pub in_ok: bool,
    pub out_ok: bool,
    pub input_beats: u64,
}

impl From<&BridgeSnapshot> for StateDto {
    fn from(snap: &BridgeSnapshot) -> Self {
        StateDto {
            bpm: snap.state.bpm,
            beat: snap.state.beat,
            playing: snap.state.playing,
            in_ok: snap.state.in_ok && snap.in_port_ok,
            out_ok: snap.state.playing && snap.out_port_ok,
            input_beats: snap.state.input_beats,
        }
    }
}

#[derive(Deserialize, Clone, Copy, Debug)]
#[serde(rename_all = "lowercase")]
pub enum DirectionArg {
    Push,
    Slow,
}

#[tauri::command]
pub fn play(bridge: State<Bridge>) {
    bridge.command(Command::Play);
}

#[tauri::command]
pub fn pause(bridge: State<Bridge>) {
    bridge.command(Command::Pause);
}

#[tauri::command]
pub fn nudge_start(bridge: State<Bridge>, direction: DirectionArg) {
    let direction = match direction {
        DirectionArg::Push => Direction::Push,
        DirectionArg::Slow => Direction::Slow,
    };
    bridge.command(Command::NudgeStart(direction));
}

#[tauri::command]
pub fn nudge_stop(bridge: State<Bridge>) {
    bridge.command(Command::NudgeStop);
}

#[tauri::command]
pub fn get_settings(store: State<SettingsStore>) -> Settings {
    store
        .current
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
}

/// Validates, applies and persists the settings; nothing changes if any step fails.
#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    bridge: State<Bridge>,
    store: State<SettingsStore>,
    settings: Settings,
) -> Result<Settings, String> {
    let new = settings::validate(settings)?;
    let mut current = store
        .current
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    let autostart = app.autolaunch();
    let enabled = autostart.is_enabled().unwrap_or(false);
    if new.launch_at_login != enabled {
        let result = if new.launch_at_login {
            autostart.enable()
        } else {
            autostart.disable()
        };
        result.map_err(|e| format!("Could not change launch at login: {e}"))?;
    }
    settings::save(&store.path, &new).map_err(|e| format!("Could not save settings: {e}"))?;

    if new.input_name != current.input_name || new.output_name != current.output_name {
        bridge.rename_ports(&new.input_name, &new.output_name);
    }
    bridge.command(Command::SetNudgeStrength(new.nudge_strength_pct));
    current.clone_from(&new);
    Ok(new)
}

#[tauri::command]
pub fn quit(app: AppHandle) {
    app.exit(0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use clock_core::State as EngineState;

    fn snap(playing: bool, in_ok: bool, ports: (bool, bool)) -> BridgeSnapshot {
        BridgeSnapshot {
            state: EngineState {
                bpm: Some(120.0),
                beat: playing.then_some(1),
                playing,
                in_ok,
                input_beats: 7,
            },
            in_port_ok: ports.0,
            out_port_ok: ports.1,
        }
    }

    #[test]
    fn state_dto_out_ok_requires_playing_and_port() {
        assert!(StateDto::from(&snap(true, true, (true, true))).out_ok);
        assert!(!StateDto::from(&snap(false, true, (true, true))).out_ok);
        assert!(!StateDto::from(&snap(true, true, (true, false))).out_ok);
    }

    #[test]
    fn state_dto_in_ok_requires_signal_and_port() {
        assert!(StateDto::from(&snap(false, true, (true, true))).in_ok);
        assert!(!StateDto::from(&snap(false, false, (true, true))).in_ok);
        assert!(!StateDto::from(&snap(false, true, (false, true))).in_ok);
    }

    #[test]
    fn state_dto_serializes_camel_case() {
        let json = serde_json::to_value(StateDto::from(&snap(true, true, (true, true)))).unwrap();
        assert_eq!(json["inputBeats"], 7);
        assert_eq!(json["inOk"], true);
    }
}
