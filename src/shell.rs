use anyhow::{Context, Result};
use directories::BaseDirs;
use std::fs::OpenOptions;
use std::io::Write;

const ZSH_SCRIPT: &str = include_str!("../shell/chitin.zsh");

pub fn install() -> Result<()> {
    // 1. Determine install location (XDG Data Home or similar)
    let base_dirs = BaseDirs::new().context("Could not determine base directories")?;
    // Use ~/.local/share/chitin/chitin.zsh (Linux/macOS standard)
    let data_dir = base_dirs.data_dir().join("chitin");

    if !data_dir.exists() {
        std::fs::create_dir_all(&data_dir)?;
    }

    let script_path = data_dir.join("chitin.zsh");
    std::fs::write(&script_path, ZSH_SCRIPT)?;
    println!("Installed shell plugin to {:?}", script_path);

    // 2. Update .zshrc
    let home_dir = base_dirs.home_dir();
    let zshrc_path = home_dir.join(".zshrc");

    if !zshrc_path.exists() {
        // If no .zshrc, we might warn or create it. Creating is safer if user intends to use Zsh.
        // But let's check if it exists first.
        println!("Warning: No ~/.zshrc found. Creating one.");
        std::fs::File::create(&zshrc_path)?;
    }

    let source_line = format!("source \"{}\"", script_path.to_string_lossy());

    // Check if already present
    let content = std::fs::read_to_string(&zshrc_path)?;
    if content.contains(&source_line) {
        println!("Shell plugin already sourced in {:?}", zshrc_path);
        return Ok(());
    }

    // Append
    let mut file = OpenOptions::new().append(true).open(&zshrc_path)?;

    writeln!(file, "\n# Chitin Shell Integration")?;
    writeln!(file, "{}", source_line)?;

    println!("Added source line to {:?}", zshrc_path);
    println!("Please restart your shell or run 'source ~/.zshrc' to activate.");

    Ok(())
}
