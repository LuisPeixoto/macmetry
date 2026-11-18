// A release build is a menu bar app: no terminal window behind it.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    macmetry_desktop_lib::run()
}
