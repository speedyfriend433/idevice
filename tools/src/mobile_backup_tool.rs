use clap::{Arg, Command};
use idevice::{mobile_backup::{MobileBackupClient, BackupType}, IdeviceService};
use std::path::PathBuf;

mod common;

#[tokio::main]
async fn main() {
    env_logger::init();

    let matches = Command::new("mobile_backup_tool")
        .about("iOS device backup and restore tool")
        .arg(Arg::new("udid").index(1).help("Device UDID"))
        .arg(Arg::new("backup").long("backup").conflicts_with("restore"))
        .arg(Arg::new("restore").long("restore").conflicts_with("backup"))
        .arg(Arg::new("full").long("full").help("Perform full backup"))
        .arg(Arg::new("encryption-key").long("encryption-key").value_name("KEY"))
        .arg(Arg::new("target").required(true).value_name("PATH"))
        .get_matches();

    let provider = common::get_provider(
        matches.get_one::<String>("udid"),
        None,
        None,
        "mobile-backup-tool"
    ).await.unwrap();

    let mut client = MobileBackupClient::connect(&*provider).await.unwrap();
    let target = PathBuf::from(matches.get_one::<String>("target").unwrap());

    if matches.get_flag("backup") {
        let backup_type = if matches.get_flag("full") {
            BackupType::Full
        } else {
            BackupType::Incremental
        };

        client.start_backup(
            backup_type,
            &target,
            matches.get_one::<String>("encryption-key").map(|s| s.as_str())
        ).await.unwrap();
        println!("Backup initiated successfully");
    } else if matches.get_flag("restore") {
        client.start_restore(
            &target,
            matches.get_one::<String>("encryption-key").map(|s| s.as_str())
        ).await.unwrap();
        println!("Restore initiated successfully");
    }
}