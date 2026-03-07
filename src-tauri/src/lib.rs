use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{
    window::{Color, Effect, EffectsBuilder},
    Emitter, Manager,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

#[derive(Debug, Deserialize)]
struct WeatherConfig {
    #[serde(default)]
    city: Option<String>,
    #[serde(default)]
    latitude: Option<f64>,
    #[serde(default)]
    longitude: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct GeoResult {
    latitude: f64,
    longitude: f64,
}

#[derive(Debug, Deserialize)]
struct GeoResponse {
    #[serde(default)]
    results: Vec<GeoResult>,
}

#[derive(Debug, Deserialize)]
struct OpenMeteoCurrentWeather {
    is_day: u8,
    weather_code: u32,
}

#[derive(Debug, Deserialize)]
struct OpenMeteoCurrent {
    current: OpenMeteoCurrentWeather,
}

#[derive(Debug, Serialize)]
struct WeatherInfo {
    icon: String,
}

#[derive(Debug, Deserialize)]
struct MetricDefinitionFile {
    version: u32,
    metrics: Vec<MetricDefinition>,
}

#[derive(Debug, Deserialize)]
struct MetricDefinition {
    id: String,
    name: String,
    category: String,
    group: String,
    #[serde(default)]
    sub_group: Option<String>,
    unit: String,
    value_type: String,
    target_max: Option<f64>,
    target_min: Option<f64>,
    body_parts: Option<Vec<String>>,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StatusValueFile {
    version: u32,
    metrics: HashMap<String, f64>,
}

#[derive(Debug, Deserialize)]
struct UserProfile {
    username: String,
    birth_date: String,
}

#[derive(Debug, Serialize)]
struct StatusMetric {
    id: String,
    name: String,
    category: String,
    group: String,
    sub_group: Option<String>,
    unit: String,
    value_type: String,
    value: Option<f64>,
    target_max: Option<f64>,
    target_min: Option<f64>,
    body_parts: Vec<String>,
    description: Option<String>,
}

#[derive(Debug, Serialize)]
struct StatusData {
    definition_version: u32,
    value_version: u32,
    username: String,
    game_days: Option<u64>,
    bmi: Option<f64>,
    metrics: Vec<StatusMetric>,
}

fn read_json_file<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

    serde_json::from_str(&content)
        .map_err(|e| format!("Invalid JSON in {}: {}", path.display(), e))
}

fn resolve_data_dir() -> Result<PathBuf, String> {
    let cwd = std::env::current_dir().map_err(|e| format!("Cannot resolve current dir: {}", e))?;
    let candidates = [cwd.join("data"), cwd.join("..").join("data")];

    for candidate in candidates {
        if candidate.is_dir() {
            return Ok(candidate);
        }
    }

    Err("Cannot find data directory. Checked ./data and ../data".to_string())
}

fn parse_birth_date(date_str: &str) -> Result<(i32, u32, u32), String> {
    let parts: Vec<&str> = date_str.split('-').collect();
    if parts.len() != 3 {
        return Err(format!(
            "Invalid birth_date '{}'. Expected format YYYY-MM-DD",
            date_str
        ));
    }

    let year = parts[0]
        .parse::<i32>()
        .map_err(|_| format!("Invalid year in birth_date '{}'", date_str))?;
    let month = parts[1]
        .parse::<u32>()
        .map_err(|_| format!("Invalid month in birth_date '{}'", date_str))?;
    let day = parts[2]
        .parse::<u32>()
        .map_err(|_| format!("Invalid day in birth_date '{}'", date_str))?;

    if !(1..=12).contains(&month) {
        return Err(format!("Invalid month '{}' in birth_date '{}'", month, date_str));
    }
    if !(1..=31).contains(&day) {
        return Err(format!("Invalid day '{}' in birth_date '{}'", day, date_str));
    }

    Ok((year, month, day))
}

fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let y = year - if month <= 2 { 1 } else { 0 };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = month as i32 + if month > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + day as i32 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    (era * 146097 + doe) as i64
}

fn calculate_game_days(birth_date: &str) -> Result<u64, String> {
    let (year, month, day) = parse_birth_date(birth_date)?;
    let birth_days = days_from_civil(year, month, day) - days_from_civil(1970, 1, 1);

    let now_duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("System clock before UNIX_EPOCH: {}", e))?;
    let today_days = (now_duration.as_secs() / 86_400) as i64;

    let diff = today_days - birth_days;
    Ok(if diff > 0 { diff as u64 } else { 0 })
}

fn calculate_bmi(values: &HashMap<String, f64>) -> Option<f64> {
    let weight = values.get("weight_kg")?;
    let height_cm = values.get("height_cm")?;
    if *height_cm <= 0.0 {
        return None;
    }

    let height_m = height_cm / 100.0;
    Some(weight / (height_m * height_m))
}

#[tauri::command]
fn load_status_data() -> Result<StatusData, String> {
    let data_dir = resolve_data_dir()?;
    let definitions_path = data_dir.join("status_metric_definitions.json");
    let values_path = data_dir.join("status.json");
    let user_profile_path = data_dir.join("user_profile.json");

    let definitions: MetricDefinitionFile = read_json_file(&definitions_path)?;
    let values: StatusValueFile = read_json_file(&values_path)?;
    let user_profile: UserProfile = read_json_file(&user_profile_path)?;

    let mut metric_ids = HashSet::new();
    for metric in &definitions.metrics {
        if !metric_ids.insert(metric.id.clone()) {
            return Err(format!("Duplicate metric id found in definitions: {}", metric.id));
        }

        if metric.value_type != "number" {
            return Err(format!(
                "Unsupported value_type '{}' for metric '{}'. Only 'number' is supported in MVP.",
                metric.value_type, metric.id
            ));
        }
    }

    for value_id in values.metrics.keys() {
        if !metric_ids.contains(value_id) {
            return Err(format!(
                "Metric '{}' exists in status.json but is missing in status_metric_definitions.json",
                value_id
            ));
        }
    }

    let merged_metrics = definitions
        .metrics
        .into_iter()
        .map(|metric| StatusMetric {
            value: values.metrics.get(&metric.id).copied(),
            id: metric.id,
            name: metric.name,
            category: metric.category,
            group: metric.group,
            sub_group: metric.sub_group,
            unit: metric.unit,
            value_type: metric.value_type,
            target_max: metric.target_max,
            target_min: metric.target_min,
            body_parts: metric.body_parts.unwrap_or_default(),
            description: metric.description,
        })
        .collect();

    Ok(StatusData {
        definition_version: definitions.version,
        value_version: values.version,
        username: user_profile.username,
        game_days: Some(calculate_game_days(&user_profile.birth_date)?),
        bmi: calculate_bmi(&values.metrics),
        metrics: merged_metrics,
    })
}

/// Map WMO weather code + is_day to OpenWeatherMap-style icon name.
/// See https://open-meteo.com/en/docs for WMO codes.
fn wmo_to_icon(code: u32, is_day: bool) -> &'static str {
    match code {
        0 => if is_day { "01d" } else { "01n" },           // Clear sky
        1 | 2 => if is_day { "02d" } else { "02n" },       // Mainly clear / Partly cloudy
        3 => if is_day { "03d" } else { "03n" },            // Overcast
        45 | 48 => if is_day { "50d" } else { "50n" },      // Fog
        51 | 53 | 55 => if is_day { "09d" } else { "09n" }, // Drizzle
        56 | 57 => if is_day { "09d" } else { "09n" },      // Freezing drizzle
        61 | 63 => if is_day { "10d" } else { "10n" },      // Rain slight/moderate
        65 => if is_day { "09d" } else { "09n" },            // Rain heavy
        66 | 67 => if is_day { "13d" } else { "13n" },      // Freezing rain
        71 | 73 | 75 | 77 => if is_day { "13d" } else { "13n" }, // Snow
        80 | 81 | 82 => if is_day { "09d" } else { "09n" }, // Rain showers
        85 | 86 => if is_day { "13d" } else { "13n" },      // Snow showers
        95 | 96 | 99 => if is_day { "11d" } else { "11n" }, // Thunderstorm
        _ => if is_day { "02d" } else { "02n" },
    }
}

#[tauri::command]
async fn get_weather() -> Result<WeatherInfo, String> {
    let data_dir = resolve_data_dir()?;
    let config_path = data_dir.join("weather.json");
    let config: WeatherConfig = read_json_file(&config_path)?;

    // Resolve lat/lon: use explicit coords, or geocode from city name
    let (lat, lon) = if let (Some(lat), Some(lon)) = (config.latitude, config.longitude) {
        (lat, lon)
    } else if let Some(city) = &config.city {
        if city.is_empty() {
            return Err("Weather not configured: set city or latitude/longitude in data/weather.json".to_string());
        }
        let geo_url = format!(
            "https://geocoding-api.open-meteo.com/v1/search?name={}&count=1",
            city
        );
        let geo_resp: GeoResponse = reqwest::get(&geo_url)
            .await
            .map_err(|e| format!("Geocoding request failed: {}", e))?
            .json()
            .await
            .map_err(|e| format!("Failed to parse geocoding response: {}", e))?;
        let loc = geo_resp.results.first()
            .ok_or_else(|| format!("City '{}' not found", city))?;
        (loc.latitude, loc.longitude)
    } else {
        return Err("Weather not configured: set city or latitude/longitude in data/weather.json".to_string());
    };

    let weather_url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&current=weather_code,is_day",
        lat, lon
    );

    let weather: OpenMeteoCurrent = reqwest::get(&weather_url)
        .await
        .map_err(|e| format!("Weather request failed: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse weather response: {}", e))?;

    let icon = wmo_to_icon(weather.current.weather_code, weather.current.is_day != 0).to_string();

    Ok(WeatherInfo { icon })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();

            let _ = window.set_decorations(false);
            let _ = window.set_shadow(false);
            let _ = window.set_effects(
                EffectsBuilder::new()
                    .effect(Effect::Acrylic)
                    .color(Color(140, 0, 18, 110))
                    .build(),
            );

            // 注册全局快捷键 Cmd+Shift+R (macOS) 或 Ctrl+Shift+R (Windows/Linux)
            #[cfg(target_os = "macos")]
            let shortcut = "Command+Shift+R";

            #[cfg(not(target_os = "macos"))]
            let shortcut = "Ctrl+Shift+R";

            app.global_shortcut().on_shortcut(shortcut, move |_app, _shortcut, event| {
                // 只在按键按下时触发，避免释放时也触发
                if event.state == ShortcutState::Pressed {
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.set_always_on_top(false);
                        let _ = window.hide();
                    } else {
                        // 展开到主显示器全屏
                        if let Ok(Some(monitor)) = window.primary_monitor() {
                            let size = monitor.size();
                            let pos = monitor.position();
                            let _ = window.set_position(tauri::PhysicalPosition::new(pos.x, pos.y));
                            let _ = window.set_size(tauri::PhysicalSize::new(size.width, size.height));
                        }
                        let _ = window.set_always_on_top(true);
                        let _ = window.show();
                        let _ = window.set_focus();
                        let _ = window.emit("reality://summoned", ());
                    }
                }
            })?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![load_status_data, get_weather])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
