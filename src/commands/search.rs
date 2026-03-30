use geo::{Distance, Haversine, point};

use crate::cache::Cache;
use crate::geocode;
use crate::output::{self, FieldFilter, OutputFormat};
use crate::spot::Spot;

pub struct SearchArgs {
    pub term: String,
    pub near: Option<String>,
    pub radius: f64,
    pub limit: usize,
    pub output: Option<OutputFormat>,
    pub fields: Option<String>,
}

pub fn run(args: &SearchArgs) -> anyhow::Result<()> {
    let cache = Cache::new();
    cache.ensure_cache()?;
    let mut spots = cache.load_spots()?;

    spots.retain(|s| s.matches_text(&args.term));

    if let Some(ref location) = args.near {
        let (center_lat, center_lon) = geocode::resolve_location(location)?;
        let center = point!(x: center_lon, y: center_lat);

        for spot in &mut spots {
            let p = point!(x: spot.lon, y: spot.lat);
            spot.distance_km = Some(Haversine.distance(center, p) / 1000.0);
        }
        spots.retain(|s| s.distance_km.is_some_and(|d| d <= args.radius));
        spots.sort_by(|a, b| {
            a.distance_km
                .partial_cmp(&b.distance_km)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    let total_count = spots.len();
    let limited: Vec<Spot> = spots.into_iter().take(args.limit).collect();

    let format = OutputFormat::resolve(args.output);
    let filter = args
        .fields
        .as_deref()
        .map(FieldFilter::parse::<Spot>)
        .transpose()?;
    output::format_list(&limited, total_count, format, filter.as_ref())
}
