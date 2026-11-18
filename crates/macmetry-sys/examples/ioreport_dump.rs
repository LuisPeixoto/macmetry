//! Lists every IOReport channel this Mac publishes (group, subgroup, name, unit, states) as JSON lines, and the HID
//! temperature sensors. `cargo run -p macmetry-sys --example ioreport_dump > ioreport.jsonl`. Read-only.
//! Run it on an Apple silicon Mac and commit the output as a fixture to extend the Apple silicon collectors.

use macmetry_sys::{hid, ioreport::Subscription};
use serde_json::json;

fn main() {
    match Subscription::all().and_then(|s| s.sample().map(|sample| s.read(&sample))) {
        Ok(channels) => {
            for c in channels {
                println!(
                    "{}",
                    json!({ "kind": "channel", "group": c.group, "subgroup": c.sub_group, "name": c.name, "unit": c.unit,
                            "value": c.value, "states": c.states.iter().map(|(n, r)| json!([n, r])).collect::<Vec<_>>() })
                );
            }
        }
        Err(e) => eprintln!("IOReport: {e}"),
    }
    for t in hid::read_temperatures() {
        println!("{}", json!({ "kind": "hid_temperature", "name": t.label, "celsius": t.celsius }));
    }
}
