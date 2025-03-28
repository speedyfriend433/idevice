// Jackson Coxson
// idevice Rust implementation of House Arrest functionality

use clap::{Arg, Command};
use idevice::{house_arrest::HouseArrestClient, IdeviceService};

mod common;

#[tokio::main]
async fn main() {
    env_logger::init();

    let matches = Command::new("house_arrest_tool")
        .about("Access app containers on iOS devices")
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
            Arg::new("list")
                .long("list")
                .short('l')
                .help("List installed applications")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("info")
                .long("info")
                .short('i')
                .value_name("BUNDLE_ID")
                .help("Get application info"),
        )
        .arg(
            Arg::new("documents")
                .long("documents")
                .short('d')
                .value_name("BUNDLE_ID")
                .help("List files in app's Documents directory"),
        )
        .arg(
            Arg::new("container")
                .long("container")
                .short('c')
                .value_name("BUNDLE_ID")
                .help("List files in app's Container directory"),
        )
        .get_matches();

    if matches.get_flag("about") {
        println!("house_arrest_tool - access app containers on iOS devices. Reimplementation of libimobiledevice's functionality.");
        println!("Copyright (c) 2025 Jackson Coxson");
        return;
    }

    let udid = matches.get_one::<String>("udid");
    let host = matches.get_one::<String>("host");
    let pairing_file = matches.get_one::<String>("pairing_file");

    let provider =
        match common::get_provider(udid, host, pairing_file, "house-arrest-tool-jkcoxson").await {
            Ok(p) => p,
            Err(e) => {
                eprintln!("{e}");
                return;
            }
        };

    let mut house_arrest_client = match HouseArrestClient::connect(&*provider).await {
        Ok(client) => client,
        Err(e) => {
            eprintln!("Failed to connect to House Arrest service: {e:?}");
            return;
        }
    };

    if matches.get_flag("list") {
        match house_arrest_client.list_installed_applications().await {
            Ok(apps) => {
                println!("Installed applications:");
                for app in apps {
                    println!("  {}", app);
                }
            }
            Err(e) => {
                eprintln!("Failed to list applications: {e:?}");
            }
        }
    }

    if let Some(bundle_id) = matches.get_one::<String>("info") {
        match house_arrest_client.get_application_info(bundle_id).await {
            Ok(info) => {
                println!("Application info for '{}':", bundle_id);
                for (key, value) in info {
                    println!("  {}: {:?}", key, value);
                }
            }
            Err(e) => {
                eprintln!("Failed to get application info: {e:?}");
            }
        }
    }

    if let Some(bundle_id) = matches.get_one::<String>("documents") {
        match house_arrest_client.documents(bundle_id).await {
            Ok(mut afc_client) => {
                match afc_client.read_directory("/").await {
                    Ok(entries) => {
                        println!("Files in Documents directory of '{}':", bundle_id);
                        for entry in entries {
                            println!("  {}", entry);
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to list files: {e:?}");
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to access Documents directory: {e:?}");
            }
        }
    }

    if let Some(bundle_id) = matches.get_one::<String>("container") {
        match house_arrest_client.container(bundle_id).await {
            Ok(mut afc_client) => {
                match afc_client.read_directory("/").await {
                    Ok(entries) => {
                        println!("Files in Container directory of '{}':", bundle_id);
                        for entry in entries {
                            println!("  {}", entry);
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to list files: {e:?}");
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to access Container directory: {e:?}");
            }
        }
    }
}