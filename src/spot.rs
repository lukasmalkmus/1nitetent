use serde::Serialize;

use crate::output::{DetailView, FieldNames, Tabular};

#[derive(Debug, Clone, Serialize)]
pub struct Spot {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub description: String,
    pub link: String,
    pub location: String,
    pub lat: f64,
    pub lon: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance_km: Option<f64>,
}

impl Spot {
    pub fn strip_html(html: &str) -> String {
        let s = html
            .replace("<br />", "\n")
            .replace("<br/>", "\n")
            .replace("<br>", "\n")
            .replace("</p>", "\n")
            .replace("&amp;", "&")
            .replace("&quot;", "\"")
            .replace("&lt;", "<")
            .replace("&gt;", ">");
        let mut out = String::with_capacity(s.len());
        let mut in_tag = false;
        for ch in s.chars() {
            match ch {
                '<' => in_tag = true,
                '>' => in_tag = false,
                _ if !in_tag => out.push(ch),
                _ => {}
            }
        }
        // Normalize whitespace: collapse multiple blank lines.
        let mut result = String::with_capacity(out.len());
        let mut prev_blank = false;
        for line in out.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                if !prev_blank && !result.is_empty() {
                    result.push('\n');
                }
                prev_blank = true;
            } else {
                if prev_blank && !result.is_empty() {
                    result.push('\n');
                }
                result.push_str(trimmed);
                result.push('\n');
                prev_blank = false;
            }
        }
        result.trim().to_string()
    }

    /// Extract the numeric ID from a spot name like "#2513 Name".
    pub fn extract_id(name: &str) -> String {
        name.strip_prefix('#')
            .and_then(|s| s.split_whitespace().next())
            .unwrap_or(name)
            .to_string()
    }

    pub fn matches_id(&self, query: &str) -> bool {
        let query = query.strip_prefix('#').unwrap_or(query);
        self.id == query
    }

    pub fn matches_text(&self, term: &str) -> bool {
        let lower = term.to_lowercase();
        self.name.to_lowercase().contains(&lower)
            || self.description.to_lowercase().contains(&lower)
    }
}

impl Tabular for Spot {
    fn headers() -> &'static [&'static str] {
        &["Name", "Location", "Link", "Distance"]
    }

    fn row(&self) -> Vec<String> {
        vec![
            self.name.clone(),
            self.location.clone(),
            self.link.clone(),
            self.distance_km
                .map_or_else(String::new, |d| format!("{d:.0} km")),
        ]
    }
}

impl DetailView for Spot {
    fn fields(&self) -> Vec<(&'static str, String)> {
        let mut f = vec![
            ("Name", self.name.clone()),
            ("Location", self.location.clone()),
            ("Contact", self.link.clone()),
        ];
        if let Some(d) = self.distance_km {
            f.push(("Distance", format!("{d:.1} km")));
        }
        if !self.description.is_empty() {
            f.push(("Description", self.description.clone()));
        }
        f
    }
}

impl FieldNames for Spot {
    fn valid_fields() -> &'static [&'static str] {
        &[
            "id",
            "name",
            "description",
            "link",
            "location",
            "lat",
            "lon",
            "distance_km",
        ]
    }
}
