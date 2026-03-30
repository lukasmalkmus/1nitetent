use crate::cache::Cache;

pub fn run() -> anyhow::Result<()> {
    let cache = Cache::new();
    cache.refresh()?;
    Ok(())
}
