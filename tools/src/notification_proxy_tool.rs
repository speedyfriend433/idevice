// Jackson Coxson
// idevice Rust implementation of Notification Proxy functionality

use clap::{Arg, Command};
use idevice::{notification_proxy::{NotificationProxyClient, NotificationType}, IdeviceService};
use tokio::time::Duration;

mod common;

#[tokio::main]
async fn main() {
    env_logger::init();

    let matches = Command::new("notification_proxy_tool")
        .about("Send and receive notifications to/from iOS devices")
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
            Arg::new("post")
                .long("post")
                .short('p')
                .value_name("NOTIFICATION")
                .help("Post a notification to the device"),
        )
        .arg(
            Arg::new("observe")
                .long("observe")
                .short('o')
                .value_name("NOTIFICATION")
                .help("Observe a notification (can be specified multiple times)")
                .action(clap::ArgAction::Append),
        )
        .arg(
            Arg::new("timeout")
                .long("timeout")
                .short('t')
                .value_name("SECONDS")
                .help("Timeout in seconds (default: 60)")
                .default_value("60"),
        )
        .get_matches();

    if matches.get_flag("about") {
        println!("notification_proxy_tool - send and receive notifications to/from iOS devices. Reimplementation of libimobiledevice's functionality.");
        println!("Copyright (c) 2025 Jackson Coxson");
        return;
    }

    let udid = matches.get_one::<String>("udid");
    let host = matches.get_one::<String>("host");
    let pairing_file = matches.get_one::<String>("pairing_file");
    let timeout = matches.get_one::<String>("timeout")
        .unwrap()
        .parse::<u64>()
        .unwrap_or(60);

    let provider =
        match common::get_provider(udid, host, pairing_file, "notification-proxy-tool-jkcoxson").await {
            Ok(p) => p,
            Err(e) => {
                eprintln!("{e}");
                return;
            }
        };

    let mut notification_proxy_client = match NotificationProxyClient::connect(&*provider).await {
        Ok(client) => client,
        Err(e) => {
            eprintln!("Failed to connect to Notification Proxy service: {e:?}");
            return;
        }
    };

    // Post a notification if requested
    if let Some(notification) = matches.get_one::<String>("post") {
        let notification_type = parse_notification(notification);
        println!("Posting notification: {:?}", notification_type);
        
        match notification_proxy_client.post_notification(notification_type).await {
            Ok(_) => println!("Notification posted successfully"),
            Err(e) => eprintln!("Failed to post notification: {e:?}"),
        }
    }

    // Observe notifications if requested
    if let Some(notifications) = matches.get_many::<String>("observe") {
        let notification_types: Vec<_> = notifications
            .map(|n| parse_notification(n))
            .collect();
        
        println!("Observing notifications: {:?}", notification_types);
        
        // Observe each notification
        for notification_type in &notification_types {
            match notification_proxy_client.observe_notification(notification_type.clone()).await {
                Ok(_) => println!("Observing: {:?}", notification_type),
                Err(e) => eprintln!("Failed to observe notification: {e:?}"),
            }
        }
        
        // Start listening for notifications
        match notification_proxy_client.start_listening().await {
            Ok(mut rx) => {
                println!("Listening for notifications for {} seconds...", timeout);
                
                // Set up a timeout
                let timeout_duration = Duration::from_secs(timeout);
                let timeout_future = tokio::time::sleep(timeout_duration);
                
                tokio::pin!(timeout_future);
                
                loop {
                    tokio::select! {
                        Some(notification) = rx.recv() => {
                            println!("Received notification: {:?}", notification);
                        }
                        _ = &mut timeout_future => {
                            println!("Timeout reached");
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to start listening for notifications: {e:?}");
            }
        }
    }
}

fn parse_notification(notification: &str) -> NotificationType {
    match notification {
        "sync-will-start" => NotificationType::SyncWillStart,
        "sync-did-finish" => NotificationType::SyncDidFinish,
        "backup-will-start" => NotificationType::BackupWillStart,
        "backup-did-finish" => NotificationType::BackupDidFinish,
        "restore-will-start" => NotificationType::RestoreWillStart,
        "restore-did-finish" => NotificationType::RestoreDidFinish,
        "app-installed" => NotificationType::AppInstalled,
        "pairing-succeeded" => NotificationType::PairingSucceeded,
        "itunes-sync-will-start" => NotificationType::ITunesSyncWillStart,
        "itunes-sync-did-finish" => NotificationType::ITunesSyncDidFinish,
        "download-will-start" => NotificationType::DownloadWillStart,
        "download-did-finish" => NotificationType::DownloadDidFinish,
        _ => NotificationType::Custom(notification.to_string()),
    }
}