//! Keeps `docs/mcp-tools.md` in step with the tools the server really offers. Run
//! `UPDATE_MCP_DOCS=1 cargo test -p macmetry-mcp --test docs` after changing a tool.

use macmetry_mcp::MacmetryServer;
use serde_json::Value;
use std::fmt::Write;
use std::path::PathBuf;

fn docs_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/mcp-tools.md")
}

/// `string`, `integer`, `array of string`... from a JSON Schema property, which may be nullable or a list.
fn type_of(property: &Value) -> String {
    let name = |v: &Value| match v {
        Value::String(s) => Some(s.clone()),
        Value::Array(items) => items.iter().filter_map(Value::as_str).find(|s| *s != "null").map(str::to_string),
        _ => None,
    };
    match name(&property["type"]) {
        Some(t) if t == "array" => {
            format!("list of {}", name(&property["items"]["type"]).unwrap_or_else(|| "values".into()))
        }
        Some(t) => t,
        None => "value".into(),
    }
}

fn generate() -> String {
    let mut tools = MacmetryServer::tool_definitions();
    tools.sort_by(|a, b| a.name.cmp(&b.name));
    let mut out = String::from(
        "# MCP tools\n\n<!-- Generated from the server by `UPDATE_MCP_DOCS=1 cargo test -p macmetry-mcp --test docs`. Do not edit by hand. -->\n\n\
         Every tool only reads. Start the server with `macmetry mcp`; see [the MCP guide](mcp.md).\n\n",
    );
    for tool in &tools {
        let _ = writeln!(out, "## `{}`: {}\n", tool.name, tool.title.clone().unwrap_or_default());
        let _ = writeln!(out, "{}\n", tool.description.clone().unwrap_or_default());
        let properties = tool.input_schema.get("properties").and_then(Value::as_object);
        match properties {
            Some(props) if !props.is_empty() => {
                let required: Vec<&str> = tool
                    .input_schema
                    .get("required")
                    .and_then(Value::as_array)
                    .map(|r| r.iter().filter_map(Value::as_str).collect())
                    .unwrap_or_default();
                out.push_str("| Argument | Type | Required | What it is |\n|---|---|---|---|\n");
                for (name, property) in props {
                    let description = property["description"].as_str().unwrap_or("").replace('\n', " ");
                    let _ = writeln!(
                        out,
                        "| `{name}` | {} | {} | {description} |",
                        type_of(property),
                        if required.contains(&name.as_str()) { "yes" } else { "no" }
                    );
                }
                out.push('\n');
            }
            _ => out.push_str("No arguments.\n\n"),
        }
    }
    out
}

#[test]
fn the_reference_matches_the_tools() {
    let expected = generate();
    let path = docs_path();
    if std::env::var_os("UPDATE_MCP_DOCS").is_some() {
        std::fs::write(&path, &expected).expect("write the reference");
        return;
    }
    let current = std::fs::read_to_string(&path).unwrap_or_default();
    assert_eq!(
        current, expected,
        "docs/mcp-tools.md is out of date. Run: UPDATE_MCP_DOCS=1 cargo test -p macmetry-mcp --test docs"
    );
}

#[test]
fn every_tool_is_in_the_reference_with_its_arguments() {
    let text = generate();
    for tool in MacmetryServer::tool_definitions() {
        assert!(text.contains(&format!("## `{}`", tool.name)));
    }
    assert!(text.contains("| `sections` | list of string | no |"), "{text}");
    assert!(text.contains("| `metric` | string | yes |"));
}
