use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct WeatherConfig {
    #[serde(default)]
    pub city: Option<String>,
    #[serde(default)]
    pub latitude: Option<f64>,
    #[serde(default)]
    pub longitude: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct GeoResult {
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Debug, Deserialize)]
pub struct GeoResponse {
    #[serde(default)]
    pub results: Vec<GeoResult>,
}

#[derive(Debug, Deserialize)]
pub struct OpenMeteoCurrentWeather {
    pub is_day: u8,
    pub weather_code: u32,
}

#[derive(Debug, Deserialize)]
pub struct OpenMeteoCurrent {
    pub current: OpenMeteoCurrentWeather,
}

#[derive(Debug, Serialize)]
pub struct WeatherInfo {
    pub icon: String,
}
