use std::io::IsTerminal as _;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};

mod cache;
mod commands;
mod geocode;
mod output;
mod spot;

use output::{FieldFilterError, OutputFormat};

#[derive(Parser)]
#[command(
    name = "1nt",
    version,
    about = "Query free one-night camping spots from 1nitetent.com"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// Output errors as JSON to stderr.
    #[arg(long, global = true, env = "ONT_JSON_ERRORS")]
    json_errors: bool,
}

#[derive(Subcommand)]
enum Command {
    /// Find spots near a location.
    Near {
        /// Place name or lat,lon coordinates.
        location: String,

        /// Search radius in kilometers.
        #[arg(long, default_value = "50")]
        radius: f64,

        /// Filter results by text in name/description.
        #[arg(long)]
        search: Option<String>,

        /// Maximum number of results.
        #[arg(long, default_value = "30")]
        limit: usize,

        #[command(flatten)]
        output: OutputArgs,
    },

    /// Search spots by text.
    Search {
        /// Search term (case-insensitive, matches name and description).
        term: String,

        /// Also filter by proximity to a location.
        #[arg(long)]
        near: Option<String>,

        /// Radius for --near filter in kilometers.
        #[arg(long, default_value = "50")]
        radius: f64,

        /// Maximum number of results.
        #[arg(long, default_value = "30")]
        limit: usize,

        #[command(flatten)]
        output: OutputArgs,
    },

    /// Show details for a specific spot.
    Spot {
        /// Spot ID (e.g., 2513 or #2513).
        id: String,

        #[command(flatten)]
        output: OutputArgs,
    },

    /// List all spots.
    List {
        /// Maximum number of results.
        #[arg(long, default_value = "30")]
        limit: usize,

        #[command(flatten)]
        output: OutputArgs,
    },

    /// Force re-download and re-enrich all data.
    Refresh,

    /// Print version and cache status.
    Version,
}

#[derive(Args)]
struct OutputArgs {
    /// Output format: table, json, ndjson.
    #[arg(short, long)]
    output: Option<OutputFormat>,

    /// Comma-separated list of fields to include in output.
    #[arg(short = 'F', long)]
    fields: Option<String>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let json_errors = cli.json_errors || !std::io::stderr().is_terminal();

    if let Err(err) = run(cli) {
        let code = exit_code(&err);
        if json_errors {
            output::print_json_error(&err, error_code(&err));
        } else {
            eprintln!("Error: {err:#}");
        }
        return ExitCode::from(code);
    }

    ExitCode::SUCCESS
}

fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Command::Near {
            location,
            radius,
            search,
            limit,
            output,
        } => commands::near::run(&commands::near::NearArgs {
            location,
            radius,
            search,
            limit,
            output: output.output,
            fields: output.fields,
        }),

        Command::Search {
            term,
            near,
            radius,
            limit,
            output,
        } => commands::search::run(&commands::search::SearchArgs {
            term,
            near,
            radius,
            limit,
            output: output.output,
            fields: output.fields,
        }),

        Command::Spot { id, output } => commands::spot::run(&commands::spot::SpotArgs {
            id,
            output: output.output,
            fields: output.fields,
        }),

        Command::List { limit, output } => commands::list::run(&commands::list::ListArgs {
            limit,
            output: output.output,
            fields: output.fields,
        }),

        Command::Refresh => commands::refresh::run(),

        Command::Version => {
            commands::version::run();
            Ok(())
        }
    }
}

fn exit_code(err: &anyhow::Error) -> u8 {
    if err.downcast_ref::<FieldFilterError>().is_some() {
        return 2;
    }
    if err.downcast_ref::<reqwest::Error>().is_some() {
        return 4;
    }
    // "not found" errors contain the phrase.
    let msg = format!("{err:#}");
    if msg.contains("not found") {
        return 3;
    }
    if msg.contains("cache") {
        return 5;
    }
    1
}

fn error_code(err: &anyhow::Error) -> &'static str {
    if err.downcast_ref::<FieldFilterError>().is_some() {
        return "usage_error";
    }
    if err.downcast_ref::<reqwest::Error>().is_some() {
        return "network_error";
    }
    let msg = format!("{err:#}");
    if msg.contains("not found") {
        return "not_found";
    }
    if msg.contains("cache") {
        return "cache_error";
    }
    "general_error"
}
