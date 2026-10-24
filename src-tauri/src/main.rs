#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, USER_AGENT};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    io::{BufRead, BufReader, Write},
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
const MIN_REFRESH_SECONDS: u64 = 300;

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
            refresh_seconds: MIN_REFRESH_SECONDS,
            launch_at_login: false,
            tray_display: "session".into(),
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct UiState {
    usage: Option<Value>,
    history: Vec<UsageSample>,
    last_error: Option<String>,
    refreshing: bool,
    settings: Settings,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UsageSample {
    timestamp: u64,
    five_hour: Option<u64>,
    seven_day: Option<u64>,
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
    value.refresh_seconds = value.refresh_seconds.clamp(MIN_REFRESH_SECONDS, 3600);
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

fn history_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(config_dir(app)?.join("usage-history.json"))
}

fn history_import_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(config_dir(app)?.join("usage-history-import.jsonl"))
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

fn usage_bar(percent: Option<u64>) -> String {
    let Some(percent) = percent else {
        return "□□□□□□□□□□".to_owned();
    };
    let filled = ((percent.min(100) + 5) / 10) as usize;
    format!("{}{}", "■".repeat(filled), "□".repeat(10 - filled))
}

fn tooltip_metric(label: &str, percent: Option<u64>) -> String {
    let value = percent.map_or("—".to_owned(), |value| format!("{value}%"));
    format!("{label} {value} [{}]", usage_bar(percent))
}

fn update_tray_title(app: &AppHandle, state: &UiState) {
    let Some(tray) = app.tray_by_id("main") else {
        return;
    };
    let title = match (&state.usage, state.settings.tray_display.as_str()) {
        (Some(usage), "week") => usage_percent(usage, "seven_day").map(|p| format!("W {p}%")),
        (Some(usage), "both") => Some(format!(
            "5H {}% · W {}%",
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
            "{}\n{}",
            tooltip_metric("5H Sessão", usage_percent(usage, "five_hour")),
            tooltip_metric("W Semana", usage_percent(usage, "seven_day"))
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
            let timestamp = now_millis();
            if let Ok(path) = cache_path(&app) {
                let cache = serde_json::json!({ "fetchedAt": timestamp, "usage": usage });
                let _ = write_json(&path, &cache);
            }
            let state = app.state::<SharedState>();
            let mut state = state.0.lock().unwrap_or_else(|e| e.into_inner());
            state.history.push(UsageSample {
                timestamp,
                five_hour: usage_percent(&usage, "five_hour"),
                seven_day: usage_percent(&usage, "seven_day"),
            });
            if let Ok(path) = history_path(&app) {
                let _ = write_json(&path, &state.history);
            }
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

fn load_usage_history(app: &AppHandle) -> Vec<UsageSample> {
    let mut samples = history_path(app)
        .ok()
        .and_then(|path| fs::read(path).ok())
        .and_then(|bytes| serde_json::from_slice::<Vec<UsageSample>>(&bytes).ok())
        .unwrap_or_default()
        .into_iter()
        .filter(valid_sample)
        .collect::<Vec<_>>();
    samples.sort_by_key(|sample| sample.timestamp);
    samples
}

fn valid_sample(sample: &UsageSample) -> bool {
    sample.timestamp > 0 && sample.timestamp <= now_millis()
}

#[tauri::command]
fn export_usage_history(app: AppHandle) -> Vec<UsageSample> {
    snapshot(&app).history
}

#[tauri::command]
fn begin_history_import(app: AppHandle) -> Result<(), String> {
    let path = history_import_path(&app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(path, b"").map_err(|e| e.to_string())
}

#[tauri::command]
fn append_history_import(app: AppHandle, samples: Vec<UsageSample>) -> Result<(), String> {
    let path = history_import_path(&app)?;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    for sample in samples {
        let json = serde_json::to_vec(&sample).map_err(|e| e.to_string())?;
        file.write_all(&json).map_err(|e| e.to_string())?;
        file.write_all(b"\n").map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn finish_history_import(app: AppHandle) -> Result<usize, String> {
    let import_path = history_import_path(&app)?;
    let mut merged = BTreeMap::new();
    let file = fs::File::open(&import_path).map_err(|e| e.to_string())?;
    for line in BufReader::new(file).lines() {
        let line = line.map_err(|e| e.to_string())?;
        let sample = serde_json::from_str::<UsageSample>(&line).map_err(|e| e.to_string())?;
        if valid_sample(&sample) {
            merged.insert(sample.timestamp, sample);
        }
    }
    {
        let state = app.state::<SharedState>();
        let mut state = state.0.lock().unwrap_or_else(|e| e.into_inner());
        for sample in state.history.iter().filter(|sample| valid_sample(sample)) {
            merged.entry(sample.timestamp).or_insert_with(|| sample.clone());
        }
        state.history = merged.into_values().collect();
        write_json(&history_path(&app)?, &state.history)?;
    }
    let _ = fs::remove_file(import_path);
    Ok(snapshot(&app).history.len())
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
            refresh_usage,
            export_usage_history,
            begin_history_import,
            append_history_import,
            finish_history_import
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();
            let settings = load_settings(&app_handle);
            let usage = load_cached_usage(&app_handle);
            let history = load_usage_history(&app_handle);
            app.manage(SharedState(Mutex::new(UiState {
                usage,
                history,
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
                .icon_as_template(cfg!(target_os = "macos"))
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
