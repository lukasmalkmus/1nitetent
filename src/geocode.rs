use anyhow::Context as _;
use serde::Deserialize;

const NOMINATIM_URL: &str = "https://nominatim.openstreetmap.org/search";

pub enum Location {
    Coords(f64, f64),
    Place(String),
}

pub fn parse_location(input: &str) -> Location {
    if let Some((lat_str, lon_str)) = input.split_once(',')
        && let (Ok(lat), Ok(lon)) = (lat_str.trim().parse::<f64>(), lon_str.trim().parse::<f64>())
    {
        return Location::Coords(lat, lon);
    }
    Location::Place(input.to_string())
}

pub fn resolve_location(input: &str) -> anyhow::Result<(f64, f64)> {
    match parse_location(input) {
        Location::Coords(lat, lon) => Ok((lat, lon)),
        Location::Place(name) => forward_geocode(&name),
    }
}

fn forward_geocode(place: &str) -> anyhow::Result<(f64, f64)> {
    eprintln!("Geocoding '{place}'...");

    let client = reqwest::blocking::Client::builder()
        .user_agent("1nt-cli")
        .build()?;

    let resp = client
        .get(NOMINATIM_URL)
        .query(&[("q", place), ("format", "json"), ("limit", "1")])
        .send()
        .context("Nominatim request failed")?
        .error_for_status()
        .context("Nominatim returned an error")?;

    let results: Vec<NominatimResult> =
        resp.json().context("failed to parse Nominatim response")?;

    let first = results
        .into_iter()
        .next()
        .with_context(|| format!("could not geocode '{place}'"))?;

    let lat: f64 = first
        .lat
        .parse()
        .context("invalid latitude from Nominatim")?;
    let lon: f64 = first
        .lon
        .parse()
        .context("invalid longitude from Nominatim")?;

    eprintln!("Resolved to {lat:.4}, {lon:.4}");
    Ok((lat, lon))
}

#[derive(Deserialize)]
struct NominatimResult {
    lat: String,
    lon: String,
}
