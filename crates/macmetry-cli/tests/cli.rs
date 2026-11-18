//! Runs the real `macmetry` binary. Every test points `MACMETRY_HOME` at a throwaway folder, so nothing touches the
//! history of the person running the tests.

use std::process::{Command, Output, Stdio};
use std::time::Duration;
use tempfile::TempDir;

fn macmetry(home: &TempDir, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_macmetry"))
        .args(args)
        .env("MACMETRY_HOME", home.path())
        .stdin(Stdio::null())
        .output()
        .expect("the binary runs")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn json(output: &Output) -> serde_json::Value {
    assert!(output.status.success(), "failed: {}", stderr(output));
    serde_json::from_slice(&output.stdout).expect("valid JSON on stdout")
}

#[test]
fn lists_every_metric() {
    let home = TempDir::new().unwrap();
    let out = macmetry(&home, &["metrics"]);
    assert!(out.status.success());
    let text = stdout(&out);
    for key in ["cpu.total", "memory.used_pct", "temp.max", "battery.percent", "power.cpu"] {
        assert!(text.contains(key), "`{key}` missing from:\n{text}");
    }
}

#[test]
fn bad_arguments_fail_fast_with_a_reason() {
    let home = TempDir::new().unwrap();
    // These are refused by the parser, before any collector starts, so they are quick on every Mac.
    for (args, hint) in [
        (&["watch", "--interval", "0.1"][..], "between 0.5 and 60"),
        (&["top", "--limit", "0"], "limit"),
        (&["history", "--metric", "nope"], "not a metric"),
        (&["history", "--metric", "cpu.total", "--since", "99d"], "31 days"),
        (&["export"], "--metric"),
    ] {
        let out = macmetry(&home, args);
        assert!(!out.status.success(), "{args:?} should fail");
        assert!(stderr(&out).contains(hint), "{args:?}: expected `{hint}` in:\n{}", stderr(&out));
    }
}

#[test]
fn history_without_a_recording_says_what_to_do() {
    let home = TempDir::new().unwrap();
    let out = macmetry(&home, &["history", "--metric", "cpu.total"]);
    assert_eq!(out.status.code(), Some(1));
    let message = stderr(&out);
    assert!(message.contains("no history yet") && message.contains("macmetry record"), "{message}");
    assert!(!home.path().join("history.sqlite").exists(), "reading must not create a database");
}

#[test]
fn describes_this_mac_as_json() {
    let home = TempDir::new().unwrap();
    let info = json(&macmetry(&home, &["info", "--json"]));
    assert!(info["model"].as_str().is_some_and(|m| m.contains(',')));
    assert!(info["logical_cores"].as_u64().unwrap() >= 1);
    assert!(matches!(info["arch"].as_str(), Some("x86_64" | "arm64")));
}

#[test]
fn a_snapshot_has_the_system_its_capabilities_and_the_readings() {
    let home = TempDir::new().unwrap();
    let v = json(&macmetry(&home, &["snapshot", "--json", "--wait-ms", "300", "--processes", "2"]));
    assert!(v["system"]["model"].is_string());
    assert_eq!(v["capabilities"]["cpu"], true);
    let snapshot = &v["snapshot"];
    assert!(snapshot["cpu"]["total_pct"].as_f64().unwrap() >= 0.0);
    assert!(snapshot["memory"]["total"].as_u64().unwrap() > 0);
    assert!(snapshot["processes"].as_array().unwrap().len() <= 2);
    // What the Mac lacks is reported as absent, in step with the capabilities.
    assert_eq!(snapshot["battery"].is_object(), v["capabilities"]["battery"].as_bool().unwrap());
}

#[test]
fn the_battery_is_an_object_or_null_never_an_error() {
    let home = TempDir::new().unwrap();
    let out = macmetry(&home, &["battery", "--json"]);
    let v = json(&out);
    assert!(v.is_null() || v["percent"].is_number());
}

#[test]
fn top_is_sorted_and_limited() {
    let home = TempDir::new().unwrap();
    let list = json(&macmetry(&home, &["top", "--json", "--limit", "3", "--sort", "memory"]));
    let list = list.as_array().unwrap();
    assert!(!list.is_empty() && list.len() <= 3);
    let memory: Vec<u64> = list.iter().map(|p| p["memory_bytes"].as_u64().unwrap()).collect();
    assert!(memory.windows(2).all(|w| w[0] >= w[1]), "biggest first: {memory:?}");
}

#[test]
fn watch_stops_after_the_requested_count() {
    let home = TempDir::new().unwrap();
    let out = macmetry(&home, &["watch", "--interval", "0.5", "--count", "2", "--processes", "0"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out).matches("Ctrl-C to leave").count(), 2);
}

#[test]
fn records_then_charts_and_exports_even_after_the_recorder_is_killed() {
    let home = TempDir::new().unwrap();
    let mut recorder = Command::new(env!("CARGO_BIN_EXE_macmetry"))
        .args(["record", "--interval", "0.5"])
        .env("MACMETRY_HOME", home.path())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    std::thread::sleep(Duration::from_secs(4));
    // A hard kill, as when the Mac loses power: the history must still read back.
    recorder.kill().unwrap();
    recorder.wait().unwrap();

    let history =
        json(&macmetry(&home, &["history", "--metric", "cpu.total", "--since", "5m", "--points", "4", "--json"]));
    let points = history["metrics"][0]["points"].as_array().unwrap();
    assert!(!points.is_empty(), "something was recorded");
    assert!(points.iter().all(|p| p["min"].as_f64().unwrap() <= p["max"].as_f64().unwrap()));

    let csv = macmetry(
        &home,
        &["export", "--metric", "cpu.total", "--metric", "memory.used_pct", "--from", "-5m", "--points", "4"],
    );
    assert!(csv.status.success(), "{}", stderr(&csv));
    let text = stdout(&csv);
    assert!(text.starts_with("timestamp,timestamp_ms,cpu.total,memory.used_pct\n"), "{text}");
    assert!(text.lines().count() >= 2);

    let file = home.path().join("out.csv");
    let file_arg = file.to_str().unwrap();
    assert!(macmetry(&home, &["export", "--metric", "cpu.total", "--from", "-5m", "-o", file_arg]).status.success());
    let again = macmetry(&home, &["export", "--metric", "cpu.total", "--from", "-5m", "-o", file_arg]);
    assert!(!again.status.success() && stderr(&again).contains("--force"), "an existing file is not replaced silently");
    assert!(
        macmetry(&home, &["export", "--metric", "cpu.total", "--from", "-5m", "-o", file_arg, "--force"])
            .status
            .success()
    );
    assert!(std::fs::read_to_string(&file).unwrap().starts_with("timestamp,"));
}

#[test]
fn export_refuses_a_folder_as_the_destination() {
    let home = TempDir::new().unwrap();
    let out = macmetry(&home, &["export", "--metric", "cpu.total", "-o", home.path().to_str().unwrap()]);
    assert!(!out.status.success());
}
