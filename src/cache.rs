use std::collections::HashMap;
use std::fmt::Write as _;
use std::fs;
use std::io::{BufRead as _, BufReader, Read as _};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use anyhow::{Context as _, bail};
use reverse_geocoder::ReverseGeocoder;

use crate::spot::Spot;

const GEOJSON_URL: &str =
    "https://1nitetent.com/app/themes/1nitetent/assets/json/campgrounds.geojson";
const CITIES_URL: &str = "https://download.geonames.org/export/dump/cities1000.zip";
const ADMIN1_URL: &str = "https://download.geonames.org/export/dump/admin1CodesASCII.txt";

const GEOJSON_TTL: Duration = Duration::from_hours(24);
const GEONAMES_TTL: Duration = Duration::from_hours(720); // 30 days

pub struct Cache {
    dir: PathBuf,
}

pub struct CacheStatus {
    pub geojson_age: Option<Duration>,
    pub geonames_age: Option<Duration>,
    pub spot_count: Option<usize>,
}

impl Cache {
    pub fn new() -> Self {
        let dir = std::env::var("XDG_CACHE_HOME")
            .map_or_else(
                |_| dirs_cache().unwrap_or_else(|| PathBuf::from(".cache")),
                PathBuf::from,
            )
            .join("1nt");
        Self { dir }
    }

    pub fn ensure_cache(&self) -> anyhow::Result<PathBuf> {
        fs::create_dir_all(&self.dir)
            .with_context(|| format!("failed to create cache dir: {}", self.dir.display()))?;

        let geojson_refreshed = self.ensure_geojson()?;
        let geonames_refreshed = self.ensure_geonames()?;

        if geojson_refreshed || geonames_refreshed || !self.enriched_path().exists() {
            self.enrich()?;
        }

        Ok(self.enriched_path())
    }

    pub fn refresh(&self) -> anyhow::Result<PathBuf> {
        fs::create_dir_all(&self.dir)
            .with_context(|| format!("failed to create cache dir: {}", self.dir.display()))?;

        self.fetch_geojson()?;
        self.fetch_geonames()?;
        self.enrich()?;

        Ok(self.enriched_path())
    }

    pub fn load_spots(&self) -> anyhow::Result<Vec<Spot>> {
        let path = self.enriched_path();
        let data = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let fc: geojson::FeatureCollection =
            data.parse().context("failed to parse enriched GeoJSON")?;

        let mut spots = Vec::with_capacity(fc.features.len());
        for feature in &fc.features {
            let props = feature
                .properties
                .as_ref()
                .context("feature missing properties")?;

            let name = props
                .get("name")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .to_string();

            let raw_desc = props
                .get("description")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");

            let link = props
                .get("link")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .to_string();

            let location = props
                .get("location")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .to_string();

            let (lon, lat) = extract_coords(feature)?;

            spots.push(Spot {
                id: Spot::extract_id(&name),
                name,
                description: Spot::strip_html(raw_desc),
                link,
                location,
                lat,
                lon,
                distance_km: None,
            });
        }

        Ok(spots)
    }

    pub fn status(&self) -> CacheStatus {
        CacheStatus {
            geojson_age: file_age(&self.raw_path()),
            geonames_age: file_age(&self.cities_csv_path()),
            spot_count: self.spot_count(),
        }
    }

    fn ensure_geojson(&self) -> anyhow::Result<bool> {
        if is_stale(&self.raw_path(), GEOJSON_TTL) {
            self.fetch_geojson()?;
            return Ok(true);
        }
        Ok(false)
    }

    fn ensure_geonames(&self) -> anyhow::Result<bool> {
        if is_stale(&self.cities_csv_path(), GEONAMES_TTL) {
            self.fetch_geonames()?;
            return Ok(true);
        }
        Ok(false)
    }

    fn fetch_geojson(&self) -> anyhow::Result<()> {
        eprintln!("Fetching campground data...");
        let client = reqwest::blocking::Client::builder()
            .user_agent("1nt-cli")
            .build()?;
        let resp = client.get(GEOJSON_URL).send()?.error_for_status()?;
        let bytes = resp.bytes()?;
        fs::write(self.raw_path(), &bytes)
            .with_context(|| "failed to write campgrounds.geojson")?;
        Ok(())
    }

    fn fetch_geonames(&self) -> anyhow::Result<()> {
        eprintln!("Fetching GeoNames data...");
        let client = reqwest::blocking::Client::builder()
            .user_agent("1nt-cli")
            .build()?;

        // Download and extract cities1000.zip
        let resp = client.get(CITIES_URL).send()?.error_for_status()?;
        let bytes = resp.bytes()?;
        let cursor = std::io::Cursor::new(&bytes);
        let mut archive = zip::ZipArchive::new(cursor)?;

        let mut cities_tsv = String::new();
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            if file.name().contains("cities1000.txt") {
                file.read_to_string(&mut cities_tsv)?;
                break;
            }
        }
        if cities_tsv.is_empty() {
            bail!("cities1000.txt not found in archive");
        }

        // Download admin1 codes
        let resp = client.get(ADMIN1_URL).send()?.error_for_status()?;
        let admin1_text = resp.text()?;

        // Build admin1 lookup: "CC.CODE" -> "Name"
        let admin1_map = build_admin1_map(&admin1_text);

        // Transform TSV to 6-col CSV for reverse_geocoder
        let csv = build_cities_csv(&cities_tsv, &admin1_map);
        fs::write(self.cities_csv_path(), csv).with_context(|| "failed to write cities1000.csv")?;

        Ok(())
    }

    fn enrich(&self) -> anyhow::Result<()> {
        eprintln!("Enriching spots with location names...");

        let geocoder = ReverseGeocoder::from_path(self.cities_csv_path())
            .with_context(|| "failed to load reverse geocoder from cities CSV")?;

        let raw =
            fs::read_to_string(self.raw_path()).with_context(|| "failed to read raw GeoJSON")?;
        let mut fc: geojson::FeatureCollection =
            raw.parse().context("failed to parse raw GeoJSON")?;

        for feature in &mut fc.features {
            let Ok((lon, lat)) = extract_coords(feature) else {
                continue;
            };
            let result = geocoder.search((lat, lon));
            let record = result.record;
            let location = if record.admin1.is_empty() {
                format!("{}, {}", record.name, record.cc)
            } else {
                format!("{}, {}, {}", record.name, record.admin1, record.cc)
            };
            if let Some(props) = feature.properties.as_mut() {
                props.insert("location".to_string(), serde_json::Value::String(location));
            }
        }

        let enriched = serde_json::to_string(&fc)?;
        fs::write(self.enriched_path(), enriched)
            .with_context(|| "failed to write enriched GeoJSON")?;

        let count = fc.features.len();
        eprintln!("Done. {count} spots enriched.");

        Ok(())
    }

    fn raw_path(&self) -> PathBuf {
        self.dir.join("campgrounds.geojson")
    }

    fn enriched_path(&self) -> PathBuf {
        self.dir.join("campgrounds.enriched.geojson")
    }

    fn cities_csv_path(&self) -> PathBuf {
        self.dir.join("cities1000.csv")
    }

    fn spot_count(&self) -> Option<usize> {
        let data = fs::read_to_string(self.enriched_path()).ok()?;
        let fc: geojson::FeatureCollection = data.parse().ok()?;
        Some(fc.features.len())
    }
}

fn extract_coords(feature: &geojson::Feature) -> anyhow::Result<(f64, f64)> {
    let geometry = feature.geometry.as_ref().context("missing geometry")?;
    match &geometry.value {
        geojson::GeometryValue::Point { coordinates } if coordinates.len() >= 2 => {
            Ok((coordinates[0], coordinates[1]))
        }
        _ => bail!("expected Point geometry"),
    }
}

fn is_stale(path: &Path, ttl: Duration) -> bool {
    file_age(path).is_none_or(|age| age > ttl)
}

fn file_age(path: &Path) -> Option<Duration> {
    let metadata = fs::metadata(path).ok()?;
    let modified = metadata.modified().ok()?;
    SystemTime::now().duration_since(modified).ok()
}

fn dirs_cache() -> Option<PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(|h| PathBuf::from(h).join(".cache"))
}

fn build_admin1_map(admin1_text: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let reader = BufReader::new(admin1_text.as_bytes());
    for line in reader.lines() {
        let Ok(line) = line else { continue };
        // Format: CC.Code\tName\tAsciiName\tGeoNameId
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 2 {
            map.insert(parts[0].to_string(), parts[1].to_string());
        }
    }
    map
}

fn build_cities_csv(tsv: &str, admin1_map: &HashMap<String, String>) -> String {
    let mut csv = String::from("lat,lon,name,admin1,admin2,cc\n");
    let reader = BufReader::new(tsv.as_bytes());
    for line in reader.lines() {
        let Ok(line) = line else { continue };
        let cols: Vec<&str> = line.split('\t').collect();
        // TSV columns: 0=geonameid, 1=name, 2=asciiname, 3=alternatenames,
        // 4=latitude, 5=longitude, 6=feature_class, 7=feature_code,
        // 8=country_code, 9=cc2, 10=admin1_code, 11=admin2_code, ...
        if cols.len() < 12 {
            continue;
        }
        let lat = cols[4];
        let lon = cols[5];
        let name = cols[1];
        let cc = cols[8];
        let admin1_code = format!("{cc}.{}", cols[10]);
        let admin1 = admin1_map
            .get(&admin1_code)
            .map_or(cols[10], String::as_str);
        let admin2 = cols[11];

        // Escape CSV fields that contain commas.
        let name_esc = csv_escape(name);
        let admin1_esc = csv_escape(admin1);
        let admin2_esc = csv_escape(admin2);

        let _ = writeln!(csv, "{lat},{lon},{name_esc},{admin1_esc},{admin2_esc},{cc}");
    }
    csv
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}
