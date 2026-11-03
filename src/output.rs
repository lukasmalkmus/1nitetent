use std::io::{self, IsTerminal as _, Write as _};

use comfy_table::presets::ASCII_MARKDOWN;
use comfy_table::{ContentArrangement, Table};
use serde::Serialize;
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Clone, Copy, Default, clap::ValueEnum, Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    #[default]
    Table,
    Json,
    Ndjson,
}

impl OutputFormat {
    /// Resolve the effective output format: if explicitly set, use it;
    /// otherwise default to `Table` for TTY, `Json` for piped output.
    pub fn resolve(explicit: Option<Self>) -> Self {
        explicit.unwrap_or_else(|| {
            if io::stdout().is_terminal() {
                Self::Table
            } else {
                Self::Json
            }
        })
    }
}

pub trait Tabular {
    fn headers() -> &'static [&'static str];
    fn row(&self) -> Vec<String>;
}

pub trait DetailView {
    fn fields(&self) -> Vec<(&'static str, String)>;
}

pub trait FieldNames {
    fn valid_fields() -> &'static [&'static str];
}

#[derive(Debug, Clone)]
pub struct FieldFilter {
    fields: Vec<String>,
}

#[derive(Debug, Error)]
#[error("invalid field '{field}', valid fields: {}", valid.join(", "))]
pub struct FieldFilterError {
    pub field: String,
    pub valid: Vec<String>,
}

impl FieldFilter {
    pub fn parse<T: FieldNames>(input: &str) -> Result<Self, FieldFilterError> {
        let valid = T::valid_fields();
        let mut fields: Vec<String> = Vec::new();
        for f in input.split(',').map(str::trim) {
            if !valid.contains(&f) {
                return Err(FieldFilterError {
                    field: f.to_string(),
                    valid: valid.iter().map(|&s| s.to_string()).collect(),
                });
            }
            if !fields.iter().any(|seen| seen == f) {
                fields.push(f.to_string());
            }
        }
        Ok(Self { fields })
    }

    fn filter_json_value(&self, value: &Value) -> Value {
        let selected = self
            .fields
            .iter()
            .map(|f| (f.clone(), value.get(f).cloned().unwrap_or(Value::Null)))
            .collect();
        Value::Object(selected)
    }

    fn cells(&self, value: &Value) -> Vec<String> {
        self.fields
            .iter()
            .map(|f| match value.get(f) {
                Some(Value::String(s)) => s.clone(),
                Some(other) => other.to_string(),
                None => "null".to_string(),
            })
            .collect()
    }
}

pub fn format_list<T: Tabular + Serialize>(
    items: &[T],
    total_count: usize,
    format: OutputFormat,
    filter: Option<&FieldFilter>,
) -> anyhow::Result<()> {
    write_list(&mut io::stdout().lock(), items, total_count, format, filter)
}

fn write_list<T: Tabular + Serialize>(
    out: &mut impl io::Write,
    items: &[T],
    total_count: usize,
    format: OutputFormat,
    filter: Option<&FieldFilter>,
) -> anyhow::Result<()> {
    let showing = items.len();
    let has_more = showing < total_count;

    match format {
        OutputFormat::Json => {
            let json_items: Vec<serde_json::Value> = items
                .iter()
                .map(|item| {
                    let v = serde_json::to_value(item)?;
                    Ok(filter.map_or_else(|| v.clone(), |f| f.filter_json_value(&v)))
                })
                .collect::<anyhow::Result<Vec<_>>>()?;
            let envelope = serde_json::json!({
                "results": json_items,
                "total_count": total_count,
                "showing": showing,
                "has_more": has_more,
            });
            serde_json::to_writer_pretty(&mut *out, &envelope)?;
            writeln!(out)?;
        }
        OutputFormat::Ndjson => {
            let meta = serde_json::json!({
                "_meta": true,
                "total_count": total_count,
                "showing": showing,
                "has_more": has_more,
            });
            serde_json::to_writer(&mut *out, &meta)?;
            writeln!(out)?;
            for item in items {
                let v = serde_json::to_value(item)?;
                let v = filter.map_or_else(|| v.clone(), |f| f.filter_json_value(&v));
                serde_json::to_writer(&mut *out, &v)?;
                writeln!(out)?;
            }
        }
        OutputFormat::Table => {
            let mut table = Table::new();
            table.load_style(ASCII_MARKDOWN);
            table.set_content_arrangement(ContentArrangement::Dynamic);

            if let Some(f) = filter {
                table.set_header(&f.fields);
                for item in items {
                    table.add_row(f.cells(&serde_json::to_value(item)?));
                }
            } else {
                table.set_header(T::headers());
                for item in items {
                    table.add_row(item.row());
                }
            }

            writeln!(out, "{table}")?;
            if has_more {
                writeln!(out, "\nShowing {showing} of {total_count} results")?;
            }
        }
    }
    Ok(())
}

pub fn format_detail<T: DetailView + Serialize>(
    item: &T,
    format: OutputFormat,
    filter: Option<&FieldFilter>,
) -> anyhow::Result<()> {
    write_detail(&mut io::stdout().lock(), item, format, filter)
}

fn write_detail<T: DetailView + Serialize>(
    out: &mut impl io::Write,
    item: &T,
    format: OutputFormat,
    filter: Option<&FieldFilter>,
) -> anyhow::Result<()> {
    match format {
        OutputFormat::Json | OutputFormat::Ndjson => {
            let v = serde_json::to_value(item)?;
            let v = filter.map_or_else(|| v.clone(), |f| f.filter_json_value(&v));
            if matches!(format, OutputFormat::Json) {
                serde_json::to_writer_pretty(&mut *out, &v)?;
            } else {
                serde_json::to_writer(&mut *out, &v)?;
            }
            writeln!(out)?;
        }
        OutputFormat::Table => {
            let mut table = Table::new();
            table.load_style(ASCII_MARKDOWN);
            table.set_content_arrangement(ContentArrangement::Dynamic);
            table.set_header(["Field", "Value"]);
            if let Some(f) = filter {
                let values = f.cells(&serde_json::to_value(item)?);
                for (name, value) in f.fields.iter().zip(&values) {
                    table.add_row([name, value]);
                }
            } else {
                for (name, value) in &item.fields() {
                    table.add_row([*name, value.as_str()]);
                }
            }
            writeln!(out, "{table}")?;
        }
    }
    Ok(())
}

pub fn print_json_error(error: &anyhow::Error, code: &str) {
    let msg = serde_json::json!({
        "error": format!("{error:#}"),
        "code": code,
    });
    let _ = writeln!(
        io::stderr(),
        "{}",
        serde_json::to_string(&msg).unwrap_or_default()
    );
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "tests assert on rendered output of static fixtures"
)]
mod tests {
    use super::*;
    use crate::spot::Spot;

    fn populated() -> Spot {
        Spot {
            id: "2513".to_owned(),
            name: "#2513 Waldrand".to_owned(),
            description: "Am See".to_owned(),
            link: "mailto:host@example.com".to_owned(),
            location: "Hessen".to_owned(),
            lat: 50.5,
            lon: 8.5,
            distance_km: Some(12.5),
        }
    }

    fn empty() -> Spot {
        Spot {
            id: String::new(),
            name: String::new(),
            description: String::new(),
            link: String::new(),
            location: String::new(),
            lat: 0.0,
            lon: 0.0,
            distance_km: None,
        }
    }

    fn filter(fields: &str) -> FieldFilter {
        FieldFilter::parse::<Spot>(fields).unwrap()
    }

    fn list(spots: &[Spot], format: OutputFormat, fields: Option<&str>) -> String {
        let filter = fields.map(filter);
        let mut out = Vec::new();
        write_list(&mut out, spots, spots.len(), format, filter.as_ref()).unwrap();
        String::from_utf8(out).unwrap()
    }

    fn detail(spot: &Spot, format: OutputFormat, fields: Option<&str>) -> String {
        let filter = fields.map(filter);
        let mut out = Vec::new();
        write_detail(&mut out, spot, format, filter.as_ref()).unwrap();
        String::from_utf8(out).unwrap()
    }

    fn assert_selects_only(record: &Value, field: &str, may_be_null: bool) {
        let keys: Vec<&String> = record.as_object().unwrap().keys().collect();
        assert_eq!(keys, [field], "-F {field}");
        assert!(may_be_null || !record[field].is_null(), "-F {field}");
    }

    #[test]
    fn every_field_appears_in_every_format() {
        for (spot, may_be_null) in [(populated(), false), (empty(), true)] {
            let spots = std::slice::from_ref(&spot);
            for field in Spot::valid_fields() {
                let selected = Some(*field);

                let json = list(spots, OutputFormat::Json, selected);
                let json: Value = serde_json::from_str(&json).unwrap();
                assert_selects_only(&json["results"][0], field, may_be_null);

                let ndjson = list(spots, OutputFormat::Ndjson, selected);
                let record = ndjson.lines().nth(1).unwrap();
                let record: Value = serde_json::from_str(record).unwrap();
                assert_selects_only(&record, field, may_be_null);

                let json = detail(&spot, OutputFormat::Json, selected);
                let json: Value = serde_json::from_str(&json).unwrap();
                assert_selects_only(&json, field, may_be_null);

                let ndjson = detail(&spot, OutputFormat::Ndjson, selected);
                assert_eq!(ndjson.lines().count(), 1, "-F {field}");
                let record: Value = serde_json::from_str(&ndjson).unwrap();
                assert_selects_only(&record, field, may_be_null);

                let table = list(spots, OutputFormat::Table, selected);
                let header = table.lines().next().unwrap();
                assert_eq!(header.trim_matches('|').trim(), *field);

                let table = detail(&spot, OutputFormat::Table, selected);
                let row = table.lines().nth(2).unwrap();
                assert!(row.starts_with(&format!("| {field} ")), "{row}");
            }
        }
    }

    #[test]
    fn table_cells_show_the_selected_value_or_null() {
        let spots = [populated()];
        let table = list(&spots, OutputFormat::Table, Some("distance_km"));
        assert_eq!(table.lines().nth(2), Some("| 12.5        |"));

        let spots = [empty()];
        let table = list(&spots, OutputFormat::Table, Some("distance_km"));
        assert_eq!(table.lines().nth(2), Some("| null        |"));

        let table = detail(&empty(), OutputFormat::Table, Some("description"));
        assert_eq!(table.lines().nth(2), Some("| description | null  |"));
    }

    #[test]
    fn table_columns_follow_the_requested_order_once_each() {
        let spots = [populated()];
        let table = list(&spots, OutputFormat::Table, Some("lon,id,lon"));
        assert_eq!(table.lines().next(), Some("| lon | id   |"));
    }

    #[test]
    fn selection_applies_to_an_empty_list() {
        let table = list(&[], OutputFormat::Table, Some("id,link"));
        assert_eq!(table.lines().next(), Some("| id | link |"));
    }

    #[test]
    fn selection_fills_a_missing_key_with_null() {
        let json = detail(&empty(), OutputFormat::Json, Some("distance_km"));
        let json: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(json, serde_json::json!({"distance_km": null}));
    }

    #[test]
    fn output_without_selection_keeps_its_shape() {
        let spots = [empty()];
        let table = list(&spots, OutputFormat::Table, None);
        assert_eq!(
            table.lines().next(),
            Some("| Name | Location | Link | Distance |")
        );

        let json = list(&spots, OutputFormat::Json, None);
        let json: Value = serde_json::from_str(&json).unwrap();
        let record = json["results"][0].as_object().unwrap();
        assert!(!record.contains_key("description"));
        assert!(!record.contains_key("distance_km"));

        let table = detail(&populated(), OutputFormat::Table, None);
        assert!(table.contains("| Contact "), "{table}");
    }
}
