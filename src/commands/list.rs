use crate::cache::Cache;
use crate::output::{self, FieldFilter, OutputFormat};
use crate::spot::Spot;

pub struct ListArgs {
    pub limit: usize,
    pub output: Option<OutputFormat>,
    pub fields: Option<String>,
}

pub fn run(args: &ListArgs) -> anyhow::Result<()> {
    let cache = Cache::new();
    cache.ensure_cache()?;
    let spots = cache.load_spots()?;

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
