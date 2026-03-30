use crate::cache::Cache;

pub fn run() {
    println!("1nt {}", env!("CARGO_PKG_VERSION"));

    let cache = Cache::new();
    let status = cache.status();

    if let Some(age) = status.geojson_age {
        println!("Campground data: {} old", format_duration(age));
    } else {
        println!("Campground data: not cached");
    }

    if let Some(age) = status.geonames_age {
        println!("GeoNames data: {} old", format_duration(age));
    } else {
        println!("GeoNames data: not cached");
    }

    if let Some(count) = status.spot_count {
        println!("Total spots: {count}");
    }
}

fn format_duration(d: std::time::Duration) -> String {
    let secs = d.as_secs();
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else if secs < 86_400 {
        format!("{}h", secs / 3600)
    } else {
        format!("{}d", secs / 86_400)
    }
}
