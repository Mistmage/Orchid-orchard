#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--host") {
        lilac_host::run_host();
        return;
    }
    orchard_lib::run()
}