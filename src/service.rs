use anyhow::{Context, Result};
use clap::ValueEnum;
use directories::BaseDirs;
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

pub fn install() -> Result<()> {
    let os = std::env::consts::OS;
    let service_type = detect_service_type(os);

    match service_type {
        Some(ServiceType::Launchd) => install_launchd()?,
        Some(ServiceType::Systemd) => install_systemd()?,
        Some(ServiceType::Openrc) => {
            println!("OpenRC detected. Please install manually:");
            let content = generate(ServiceType::Openrc)?;
            println!("\n{}\n", content);
            println!("1. Save this to /etc/init.d/chitin");
            println!("2. chmod +x /etc/init.d/chitin");
            println!("3. rc-update add chitin default");
            println!("4. rc-service chitin start");
        }
        None => anyhow::bail!(
            "Unsupported OS or init system. Please use 'chitin service generate <type>' manually."
        ),
    }

    Ok(())
}

fn detect_service_type(os: &str) -> Option<ServiceType> {
    if os == "macos" {
        return Some(ServiceType::Launchd);
    }
    if os == "linux" {
        // Simple check for systemd (checking if directory exists or systemctl command)
        if std::path::Path::new("/run/systemd/system").exists() || command_exists("systemctl") {
            return Some(ServiceType::Systemd);
        }
        if std::path::Path::new("/sbin/openrc-run").exists() {
            return Some(ServiceType::Openrc);
        }
    }
    None
}

fn command_exists(cmd: &str) -> bool {
    std::process::Command::new("which")
        .arg(cmd)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn install_launchd() -> Result<()> {
    let content = generate(ServiceType::Launchd)?;
    let base_dirs = BaseDirs::new().context("Could not determine base directories")?;
    let launch_agents = base_dirs.home_dir().join("Library/LaunchAgents");

    if !launch_agents.exists() {
        std::fs::create_dir_all(&launch_agents)?;
    }
    let plist_path = launch_agents.join("com.user.chitin.plist");

    std::fs::write(&plist_path, content)?;
    println!("Wrote service file to {:?}", plist_path);

    // Unload if exists (ignore error)
    let _ = std::process::Command::new("launchctl")
        .arg("unload")
        .arg(&plist_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    // Load
    let status = std::process::Command::new("launchctl")
        .arg("load")
        .arg(&plist_path)
        .status()
        .context("Failed to run launchctl load")?;

    if status.success() {
        println!("Service installed and started successfully.");
    } else {
        anyhow::bail!("Failed to start service with launchctl");
    }
    Ok(())
}

fn install_systemd() -> Result<()> {
    let content = generate(ServiceType::Systemd)?;
    let base_dirs = BaseDirs::new().context("Could not determine base directories")?;
    // For Systemd user units, standard path is ~/.config/systemd/user
    let config_dir = base_dirs.config_dir();
    let systemd_dir = config_dir.join("systemd/user");

    if !systemd_dir.exists() {
        std::fs::create_dir_all(&systemd_dir)?;
    }

    let service_path = systemd_dir.join("chitin.service");
    std::fs::write(&service_path, content)?;
    println!("Wrote service file to {:?}", service_path);

    // Daemon reload
    run_systemctl(&["--user", "daemon-reload"])?;

    // Enable and start
    run_systemctl(&["--user", "enable", "--now", "chitin"])?;

    println!("Service installed and started successfully.");
    Ok(())
}

fn run_systemctl(args: &[&str]) -> Result<()> {
    let status = std::process::Command::new("systemctl")
        .args(args)
        .status()
        .context("Failed to run systemctl")?;

    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("systemctl command failed: {:?}", args);
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
