#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, USER_AGENT};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::Mutex,
    thread,
    time::{Duration, Instant},
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, Window,
};
use tauri_plugin_autostart::ManagerExt as AutostartManagerExt;

const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Settings {
    language: String,
    refresh_seconds: u64,
    launch_at_login: bool,
    tray_display: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: "en".into(),
            refresh_seconds: 300,
            launch_at_login: false,
            tray_display: "session".into(),
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct UiState {
    usage: Option<Value>,
    last_error: Option<String>,
    refreshing: bool,
    settings: Settings,
}

struct SharedState(Mutex<UiState>);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CacheFile {
    usage: Value,
}

fn normalized_settings(mut value: Settings) -> Settings {
    if !["en", "pt", "es"].contains(&value.language.as_str()) {
        value.language = "en".into();
    }
    value.refresh_seconds = value.refresh_seconds.clamp(30, 3600);
    if !["session", "week", "both"].contains(&value.tray_display.as_str()) {
        value.tray_display = "session".into();
    }
    value
}

fn config_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path().app_config_dir().map_err(|e| e.to_string())
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(config_dir(app)?.join("settings.json"))
}

fn cache_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(config_dir(app)?.join("usage-cache.json"))
}

fn write_json(path: &PathBuf, value: &impl Serialize) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    fs::write(path, bytes).map_err(|e| e.to_string())
}

fn load_settings(app: &AppHandle) -> Settings {
    settings_path(app)
        .ok()
        .and_then(|path| fs::read(path).ok())
        .and_then(|bytes| serde_json::from_slice::<Settings>(&bytes).ok())
        .map(normalized_settings)
        .unwrap_or_default()
}

fn token_from_credentials() -> Option<String> {
    if let Ok(token) = std::env::var("CLAUDE_CODE_OAUTH_TOKEN") {
        if !token.trim().is_empty() {
            return Some(token);
        }
    }

    let credentials = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .map(|home| home.join(".claude").join(".credentials.json"))
        .and_then(|path| fs::read(path).ok())
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok());
    if let Some(value) = credentials {
        let token = value
            .pointer("/claudeAiOauth/accessToken")
            .or_else(|| value.get("accessToken"))
            .and_then(Value::as_str);
        if let Some(token) = token.filter(|token| !token.is_empty()) {
            return Some(token.to_owned());
        }
    }

    #[cfg(target_os = "macos")]
    {
        let output = Command::new("security")
            .args([
                "find-generic-password",
                "-s",
                "Claude Code-credentials",
                "-w",
            ])
            .output()
            .ok()?;
        if output.status.success() {
            let credential = String::from_utf8_lossy(&output.stdout);
            let credential = credential.trim();
            // Claude Code stores a JSON credential object in the macOS Keychain
            // on some installations, while others store the token as plain text.
            if let Ok(value) = serde_json::from_str::<Value>(credential) {
                let token = value
                    .pointer("/claudeAiOauth/accessToken")
                    .or_else(|| value.get("accessToken"))
                    .and_then(Value::as_str);
                if let Some(token) = token.filter(|token| !token.is_empty()) {
                    return Some(token.to_owned());
                }
            } else if !credential.is_empty() {
                return Some(credential.to_owned());
            }
        }
    }
    None
}

async fn fetch_usage() -> Result<Value, String> {
    let token = token_from_credentials().ok_or_else(|| {
        "Claude Code não conectado. Entre no Claude Code nesta máquina.".to_owned()
    })?;
    let authorization = HeaderValue::from_str(&format!("Bearer {token}"))
        .map_err(|_| "Credencial do Claude inválida.".to_owned())?;
    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, authorization);
    headers.insert(
        "anthropic-beta",
        HeaderValue::from_static("oauth-2025-04-20"),
    );
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static("claude-usage-tray/0.2"),
    );

    let response = reqwest::Client::builder()
        .timeout(Duration::from_secs(12))
        .default_headers(headers)
        .build()
        .map_err(|e| e.to_string())?
        .get(USAGE_URL)
        .send()
        .await
        .map_err(|e| format!("Falha ao consultar Claude: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("Claude respondeu {}", response.status()));
    }
    response
        .json::<Value>()
        .await
        .map_err(|_| "Resposta do Claude inválida.".to_owned())
}

fn snapshot(app: &AppHandle) -> UiState {
    app.state::<SharedState>()
        .0
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

fn emit_update(app: &AppHandle) {
    let state = snapshot(app);
    let _ = app.emit("usage:update", &state);
    update_tray_title(app, &state);
}

fn percent(value: Option<&Value>) -> Option<u64> {
    let number = value?.as_f64()?;
    Some(if number <= 1.0 {
        (number * 100.0).round() as u64
    } else {
        number.round() as u64
    })
}

fn usage_percent(usage: &Value, period: &str) -> Option<u64> {
    percent(usage.get(period)?.get("utilization"))
}

fn update_tray_title(app: &AppHandle, state: &UiState) {
    let Some(tray) = app.tray_by_id("main") else {
        return;
    };
    let title = match (&state.usage, state.settings.tray_display.as_str()) {
        (Some(usage), "week") => usage_percent(usage, "seven_day").map(|p| format!("W {p}%")),
        (Some(usage), "both") => Some(format!(
            "5H {}% · 7D {}%",
            usage_percent(usage, "five_hour").map_or("—".into(), |p| p.to_string()),
            usage_percent(usage, "seven_day").map_or("—".into(), |p| p.to_string())
        )),
        (Some(usage), _) => usage_percent(usage, "five_hour").map(|p| format!("5H {p}%")),
        _ => None,
    };
    let _ = tray.set_title(title.as_deref());

    let tooltip = match (&state.usage, state.settings.tray_display.as_str()) {
        (Some(usage), "week") => {
            usage_percent(usage, "seven_day").map(|p| format!("Claude Usage · Semana: {p}%"))
        }
        (Some(usage), "both") => Some(format!(
            "Claude Usage · Sessão: {}% · Semana: {}%",
            usage_percent(usage, "five_hour").map_or("—".into(), |p| p.to_string()),
            usage_percent(usage, "seven_day").map_or("—".into(), |p| p.to_string())
        )),
        (Some(usage), _) => {
            usage_percent(usage, "five_hour").map(|p| format!("Claude Usage · Sessão: {p}%"))
        }
        _ => Some("Claude Usage · Uso indisponível".to_owned()),
    };
    let _ = tray.set_tooltip(tooltip.as_deref());
}

async fn refresh_inner(app: AppHandle) {
    {
        let state = app.state::<SharedState>();
        let mut state = state.0.lock().unwrap_or_else(|e| e.into_inner());
        if state.refreshing {
            return;
        }
        state.refreshing = true;
    }
    emit_update(&app);

    match fetch_usage().await {
        Ok(usage) => {
            if let Ok(path) = cache_path(&app) {
                let cache = serde_json::json!({ "fetchedAt": now_millis(), "usage": usage });
                let _ = write_json(&path, &cache);
            }
            let state = app.state::<SharedState>();
            let mut state = state.0.lock().unwrap_or_else(|e| e.into_inner());
            state.usage = Some(usage);
            state.last_error = None;
            state.refreshing = false;
        }
        Err(error) => {
            let state = app.state::<SharedState>();
            let mut state = state.0.lock().unwrap_or_else(|e| e.into_inner());
            state.last_error = Some(error);
            state.refreshing = false;
        }
    }
    emit_update(&app);
}

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis() as u64)
}

#[tauri::command]
fn get_settings(app: AppHandle) -> Settings {
    snapshot(&app).settings
}

#[tauri::command]
fn save_settings(app: AppHandle, settings: Settings) -> Result<Settings, String> {
    let settings = normalized_settings(settings);
    write_json(&settings_path(&app)?, &settings)?;
    if settings.launch_at_login {
        app.autolaunch().enable().map_err(|e| e.to_string())?;
    } else {
        app.autolaunch().disable().map_err(|e| e.to_string())?;
    }
    {
        let state = app.state::<SharedState>();
        let mut state = state.0.lock().unwrap_or_else(|e| e.into_inner());
        state.settings = settings.clone();
    }
    emit_update(&app);
    Ok(settings)
}

#[tauri::command]
async fn refresh_usage(app: AppHandle) {
    refresh_inner(app).await;
}

fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn load_cached_usage(app: &AppHandle) -> Option<Value> {
    let path = cache_path(app).ok()?;
    let bytes = fs::read(path).ok()?;
    serde_json::from_slice::<CacheFile>(&bytes)
        .ok()
        .map(|cache| cache.usage)
}

fn hide_on_close(window: &Window, event: &tauri::WindowEvent) {
    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
        api.prevent_close();
        let _ = window.hide();
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .app_name("Claude Usage")
                .arg("--autostart")
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            refresh_usage
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();
            let settings = load_settings(&app_handle);
            let usage = load_cached_usage(&app_handle);
            app.manage(SharedState(Mutex::new(UiState {
                usage,
                last_error: None,
                refreshing: false,
                settings,
            })));

            let open = MenuItem::with_id(app, "open", "Open usage", true, None::<&str>)?;
            let refresh = MenuItem::with_id(app, "refresh", "Refresh now", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &refresh, &quit])?;
            let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray.png"))?;
            TrayIconBuilder::with_id("main")
                .icon(icon)
                .tooltip("Claude Usage")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "open" => show_main(app),
                    "refresh" => {
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move { refresh_inner(app).await });
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_main(tray.app_handle());
                    }
                })
                .build(app)?;

            if std::env::args().any(|arg| arg == "--autostart") {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            } else {
                show_main(&app_handle);
            }

            let initial = app_handle.clone();
            tauri::async_runtime::spawn(async move { refresh_inner(initial).await });

            let timer_app = app_handle.clone();
            thread::spawn(move || {
                let mut last_refresh = Instant::now();
                loop {
                    thread::sleep(Duration::from_secs(1));
                    let settings = snapshot(&timer_app).settings;
                    if last_refresh.elapsed() >= Duration::from_secs(settings.refresh_seconds) {
                        last_refresh = Instant::now();
                        let app = timer_app.clone();
                        tauri::async_runtime::spawn(async move { refresh_inner(app).await });
                    }
                }
            });
            emit_update(&app_handle);
            Ok(())
        })
        .on_window_event(hide_on_close)
        .run(tauri::generate_context!())
        .expect("error while running Claude Usage");
}
