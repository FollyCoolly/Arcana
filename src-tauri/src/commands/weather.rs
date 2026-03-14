use crate::models::weather::*;
use crate::storage::json_store::{read_json_file, resolve_data_dir};

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
pub async fn get_weather() -> Result<WeatherInfo, String> {
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
