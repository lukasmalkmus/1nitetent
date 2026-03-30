use anyhow::bail;

use crate::cache::Cache;
use crate::output::{self, FieldFilter, OutputFormat};
use crate::spot::Spot;

pub struct SpotArgs {
    pub id: String,
    pub output: Option<OutputFormat>,
    pub fields: Option<String>,
}

pub fn run(args: &SpotArgs) -> anyhow::Result<()> {
    let cache = Cache::new();
    cache.ensure_cache()?;
    let spots = cache.load_spots()?;

    let spot = spots.into_iter().find(|s| s.matches_id(&args.id));

    let Some(spot) = spot else {
        bail!("spot '{}' not found", args.id);
    };

    let format = OutputFormat::resolve(args.output);
    let filter = args
        .fields
        .as_deref()
        .map(FieldFilter::parse::<Spot>)
        .transpose()?;
    output::format_detail(&spot, format, filter.as_ref())
}
