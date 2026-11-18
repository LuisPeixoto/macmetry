//! Things an assistant can read without calling a tool. Every address is checked, so a URI can never name a file.

use crate::privacy::for_assistant;
use crate::server::{MacmetryServer, cap, json};
use macmetry_core::Metric;
use macmetry_sys::sampler::SampleOptions;
use rmcp::model::{ListResourceTemplatesResult, ListResourcesResult, ReadResourceResult, Resource, ResourceContents};

pub(crate) const GUIDE: &str = include_str!("guide.md");
const SCHEME: &str = "macmetry://";
const JSON: &str = "application/json";

pub(crate) fn list() -> ListResourcesResult {
    let r = |uri: &str, name: &str, description: &str, mime: &str| {
        Resource::new(format!("{SCHEME}{uri}"), name).with_description(description).with_mime_type(mime)
    };
    ListResourcesResult::with_all_items(vec![
        r("guide", "Guide", "How to use macmetry: the tools by question, and how to read the numbers", "text/markdown"),
        r("system", "This Mac", "The model, chip, cores, memory and macOS, and what it can report", JSON),
        r("snapshot", "Right now", "A reading of everything, taken when it is requested", JSON),
        r("metrics", "Metrics", "Every metric key the history and watches understand", JSON),
    ])
}

pub(crate) fn templates() -> ListResourceTemplatesResult {
    ListResourceTemplatesResult::with_all_items(Vec::new())
}

impl MacmetryServer {
    /// The text and type of a resource, or why it cannot be read.
    pub(crate) async fn read_uri(&self, uri: &str) -> Result<ReadResourceResult, String> {
        let path = uri.strip_prefix(SCHEME).ok_or_else(|| format!("`{uri}` is not a {SCHEME} address"))?;
        let (text, mime) = match path {
            "guide" => (GUIDE.to_string(), "text/markdown"),
            "system" => {
                let (system, capabilities, _) = self.measure_for_resource(false).await?;
                (json(&serde_json::json!({ "system": system, "can_read": capabilities }))?, JSON)
            }
            "snapshot" => {
                let (_, _, snapshot) = self.measure_for_resource(true).await?;
                (json(&for_assistant(snapshot))?, JSON)
            }
            "metrics" => (
                json(
                    &Metric::ALL
                        .iter()
                        .map(|m| serde_json::json!({ "key": m.key(), "name": m.label(), "unit": m.unit() }))
                        .collect::<Vec<_>>(),
                )?,
                JSON,
            ),
            _ => return Err(format!("there is no resource at {uri}")),
        };
        Ok(ReadResourceResult::new(vec![ResourceContents::text(cap(text, 200_000), uri).with_mime_type(mime)]))
    }

    async fn measure_for_resource(
        &self,
        processes: bool,
    ) -> Result<(macmetry_core::SystemInfo, macmetry_sys::sampler::Capabilities, macmetry_core::Snapshot), String> {
        let _ = SampleOptions::default();
        self.measure_public(std::time::Duration::from_millis(800), if processes { 5 } else { 0 }).await
    }
}
