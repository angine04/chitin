use anyhow::{Context, Result};
use clap::ValueEnum;
use std::env;

#[derive(ValueEnum, Clone, Debug)]
pub enum ServiceType {
    /// macOS launchd agent
    Launchd,
    /// Linux systemd user service
    Systemd,
    /// Linux OpenRC init script
    Openrc,
}

pub fn reload() -> Result<()> {
    // Send SIGHUP to the daemon
    // We use pkill to find the process with argument "daemon"
    let status = std::process::Command::new("pkill")
        .arg("-HUP")
        .arg("-f")
        .arg("chitin daemon")
        .status()
        .context("Failed to execute pkill")?;

    if status.success() {
        println!("Reload signal sent to chitin daemon.");
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "Failed to reload daemon (process not found?)"
        ))
    }
}

pub fn generate(service_type: ServiceType) -> Result<String> {
    let binary_path = env::current_exe()?
        .canonicalize()?
        .to_string_lossy()
        .into_owned();

    match service_type {
        ServiceType::Launchd => generate_launchd(&binary_path),
        ServiceType::Systemd => generate_systemd(&binary_path),
        ServiceType::Openrc => generate_openrc(&binary_path),
    }
}

fn generate_launchd(binary_path: &str) -> Result<String> {
    Ok(format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.user.chitin</string>
    <key>ProgramArguments</key>
    <array>
        <string>{binary_path}</string>
        <string>daemon</string>
    </array>
    <key>KeepAlive</key>
    <true/>
    <key>RunAtLoad</key>
    <true/>
    <key>StandardOutPath</key>
    <string>/tmp/chitin.log</string>
    <key>StandardErrorPath</key>
    <string>/tmp/chitin.error.log</string>
</dict>
</plist>
"#
    ))
}

fn generate_systemd(binary_path: &str) -> Result<String> {
    Ok(format!(
        r#"[Unit]
Description=Chitin AI Shell Assistant Daemon
Documentation=https://github.com/chitin-ai/chitin

[Service]
ExecStart={binary_path} daemon
Restart=always
RestartSec=5
Type=simple

[Install]
WantedBy=default.target
"#
    ))
}

fn generate_openrc(binary_path: &str) -> Result<String> {
    Ok(format!(
        r#"#!/sbin/openrc-run

name="chitin"
description="Chitin AI Shell Assistant Daemon"
command="{binary_path}"
command_args="daemon"
command_background=true
pidfile="/run/chitin.pid"

depend() {{
    need net
}}
"#
    ))
}
