//! MCP server exposing the same query surface as the `1nt` CLI.

use geo::{Distance, Haversine, point};
use rmcp::ServerHandler;
use rmcp::ServiceExt;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{Implementation, ProtocolVersion, ServerCapabilities, ServerInfo};
use rmcp::transport::stdio;
use rmcp::{tool, tool_handler, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::cache::Cache;
use crate::geocode;
use crate::spot::Spot;

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct NearArgs {
    /// Place name or `lat,lon` coordinates.
    pub location: String,
    /// Search radius in kilometers. Defaults to 50.
    #[serde(default = "default_radius")]
    pub radius: f64,
    /// Optional case-insensitive text filter against name and description.
    #[serde(default)]
    pub search: Option<String>,
    /// Maximum number of spots to return. Defaults to 30.
    #[serde(default = "default_limit")]
    pub limit: usize,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct SearchArgs {
    /// Case-insensitive search term against name and description.
    pub term: String,
    /// Optional place name or `lat,lon` coordinates to rank/filter nearby spots.
    #[serde(default)]
    pub near: Option<String>,
    /// Search radius in kilometers when `near` is set. Defaults to 50.
    #[serde(default = "default_radius")]
    pub radius: f64,
    /// Maximum number of spots to return. Defaults to 30.
    #[serde(default = "default_limit")]
    pub limit: usize,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct SpotArgs {
    /// Spot ID, with or without a leading `#`.
    pub id: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ListArgs {
    /// Maximum number of spots to return. Defaults to 30.
    #[serde(default = "default_limit")]
    pub limit: usize,
}

#[derive(Clone)]
pub struct Server {
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl Server {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "Find free one-night camping spots near a place or coordinates.",
        annotations(read_only_hint = true)
    )]
    async fn near(
        &self,
        Parameters(args): Parameters<NearArgs>,
    ) -> Result<String, rmcp::ErrorData> {
        let spots = query_near(&args).map_err(to_mcp_err)?;
        serialize_envelope(&spots)
    }

    #[tool(
        description = "Search free one-night camping spots by text, optionally near a place.",
        annotations(read_only_hint = true)
    )]
    async fn search(
        &self,
        Parameters(args): Parameters<SearchArgs>,
    ) -> Result<String, rmcp::ErrorData> {
        let spots = query_search(&args).map_err(to_mcp_err)?;
        serialize_envelope(&spots)
    }

    #[tool(
        description = "Show details for a specific camping spot ID.",
        annotations(read_only_hint = true)
    )]
    async fn spot(
        &self,
        Parameters(args): Parameters<SpotArgs>,
    ) -> Result<String, rmcp::ErrorData> {
        let spot = query_spot(&args.id).map_err(to_mcp_err)?;
        serde_json::to_string(&spot).map_err(to_mcp_err)
    }

    #[tool(
        description = "List free one-night camping spots.",
        annotations(read_only_hint = true)
    )]
    async fn list(
        &self,
        Parameters(args): Parameters<ListArgs>,
    ) -> Result<String, rmcp::ErrorData> {
        let spots = query_list(args.limit).map_err(to_mcp_err)?;
        serialize_envelope(&spots)
    }

    #[tool(
        description = "Report cache status and CLI version.",
        annotations(read_only_hint = true)
    )]
    async fn status(&self) -> Result<String, rmcp::ErrorData> {
        let cache = Cache::new();
        let status = cache.status();
        serde_json::to_string(&serde_json::json!({
            "version": env!("CARGO_PKG_VERSION"),
            "geojson_age_seconds": status.geojson_age.map(|d| d.as_secs()),
            "geonames_age_seconds": status.geonames_age.map(|d| d.as_secs()),
            "spot_count": status.spot_count,
        }))
        .map_err(to_mcp_err)
    }

    #[tool(description = "Refresh cached 1nitetent and GeoNames data.")]
    async fn refresh(&self) -> Result<String, rmcp::ErrorData> {
        let cache = Cache::new();
        let path = cache.refresh().map_err(to_mcp_err)?;
        serde_json::to_string(&serde_json::json!({
            "ok": true,
            "cache_path": path,
        }))
        .map_err(to_mcp_err)
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for Server {
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::default();
        info.protocol_version = ProtocolVersion::V_2025_06_18;
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        info.server_info = Implementation::from_build_env();
        "1nitetent".clone_into(&mut info.server_info.name);
        env!("CARGO_PKG_VERSION").clone_into(&mut info.server_info.version);
        info.instructions = Some(
            "Query free one-night camping spots from 1nitetent.com. Use `near` for location-based questions and `search` for text queries.".to_owned(),
        );
        info
    }
}

struct QueryResults {
    results: Vec<Spot>,
    total_count: usize,
    showing: usize,
}

fn query_near(args: &NearArgs) -> anyhow::Result<QueryResults> {
    let cache = Cache::new();
    cache.ensure_cache()?;
    let mut spots = cache.load_spots()?;

    let (center_lat, center_lon) = geocode::resolve_location(&args.location)?;
    let center = point!(x: center_lon, y: center_lat);

    for spot in &mut spots {
        let p = point!(x: spot.lon, y: spot.lat);
        spot.distance_km = Some(Haversine.distance(center, p) / 1000.0);
    }
    spots.retain(|s| s.distance_km.is_some_and(|d| d <= args.radius));

    if let Some(ref term) = args.search {
        spots.retain(|s| s.matches_text(term));
    }

    spots.sort_by(|a, b| {
        a.distance_km
            .partial_cmp(&b.distance_km)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(limit_results(spots, args.limit))
}

fn query_search(args: &SearchArgs) -> anyhow::Result<QueryResults> {
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

    Ok(limit_results(spots, args.limit))
}

fn query_spot(id: &str) -> anyhow::Result<Spot> {
    let cache = Cache::new();
    cache.ensure_cache()?;
    let spots = cache.load_spots()?;

    spots
        .into_iter()
        .find(|s| s.matches_id(id))
        .ok_or_else(|| anyhow::anyhow!("spot '{id}' not found"))
}

fn query_list(limit: usize) -> anyhow::Result<QueryResults> {
    let cache = Cache::new();
    cache.ensure_cache()?;
    Ok(limit_results(cache.load_spots()?, limit))
}

fn limit_results(spots: Vec<Spot>, limit: usize) -> QueryResults {
    let total_count = spots.len();
    let results: Vec<Spot> = spots.into_iter().take(limit).collect();
    let showing = results.len();
    QueryResults {
        results,
        total_count,
        showing,
    }
}

fn serialize_envelope(spots: &QueryResults) -> Result<String, rmcp::ErrorData> {
    serde_json::to_string(&serde_json::json!({
        "results": spots.results,
        "total_count": spots.total_count,
        "showing": spots.showing,
        "has_more": spots.showing < spots.total_count,
    }))
    .map_err(to_mcp_err)
}

fn default_radius() -> f64 {
    50.0
}

fn default_limit() -> usize {
    30
}

fn to_mcp_err<E: std::fmt::Display>(err: E) -> rmcp::ErrorData {
    rmcp::ErrorData::internal_error(err.to_string(), None)
}

pub async fn run() -> anyhow::Result<()> {
    let server = Server::new();
    let service = server.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
