// Jackson Coxson
// idevice Rust implementation of screenshot functionality

use clap::{Arg, Command};
use idevice::{screenshot::ScreenshotClient, IdeviceService};
use std::path::Path;

mod common;

#[tokio::main]
async fn main() {
    env_logger::init();

    let matches = Command::new("screenshot_tool")
        .about("Capture screenshots from iOS devices")
        .arg(
            Arg::new("host")
                .long("host")
                .value_name("HOST")
                .help("IP address of the device"),
        )
        .arg(
            Arg::new("pairing_file")
                .long("pairing-file")
                .value_name("PATH")
                .help("Path to the pairing file"),
        )
        .arg(
            Arg::new("udid")
                .value_name("UDID")
                .help("UDID of the device (overrides host/pairing file)")
                .index(1),
        )
        .arg(
            Arg::new("about")
                .long("about")
                .help("Show about information")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .short('o')
                .value_name("FILE")
                .help("Output file path (default: screenshot.png)")
                .default_value("screenshot.png"),
        )
        .get_matches();

    if matches.get_flag("about") {
        println!("screenshot_tool - capture screenshots from iOS devices. Reimplementation of libimobiledevice's functionality.");
        println!("Copyright (c) 2025 Jackson Coxson");
        return;
    }

    let udid = matches.get_one::<String>("udid");
    let host = matches.get_one::<String>("host");
    let pairing_file = matches.get_one::<String>("pairing_file");
    let output_path = matches.get_one::<String>("output").unwrap();

    let provider =
        match common::get_provider(udid, host, pairing_file, "screenshot-tool-jkcoxson").await {
            Ok(p) => p,
            Err(e) => {
                eprintln!("{e}");
                return;
            }
        };

    let mut screenshot_client = match ScreenshotClient::connect(&*provider).await {
        Ok(client) => client,
        Err(e) => {
            eprintln!("Failed to connect to screenshot service: {e:?}");
            return;
        }
    };

    println!("Taking screenshot...");
    match screenshot_client.save_screenshot(output_path).await {
        Ok(_) => {
            println!("Screenshot saved to: {}", output_path);
        }
        Err(e) => {
            eprintln!("Failed to take screenshot: {e:?}");
        }
    }
}