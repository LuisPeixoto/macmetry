//! Lists every SMC key this Mac has, with its type and decoded value, as JSON lines.
//! `cargo run -p macmetry-sys --example smc_dump > smc.jsonl`. Used to build the sensor catalog and to capture
//! fixtures on other Macs. Read-only.

use macmetry_sys::smc::{Smc, decode};
use serde_json::json;

fn main() {
    let smc = match Smc::open() {
        Ok(smc) => smc,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    for key in smc.keys().unwrap_or_default() {
        let Ok((kind, bytes)) = smc.read_bytes(&key) else { continue };
        let value = decode(&kind.data_type, &bytes);
        println!("{}", json!({ "key": key, "type": kind.name(), "size": kind.size, "value": value }));
    }
}
