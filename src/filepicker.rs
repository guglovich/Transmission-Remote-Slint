// src/filepicker.rs — открытие .torrent файла через XDG-совместимые диалоги

use anyhow::{Result, anyhow};

/// Открывает нативный диалог выбора файла.
/// Порядок: zenity (GNOME/любой) → kdialog (KDE) → yad → qarma.
/// Возвращает путь к выбранному .torrent файлу или Err если отменено.
pub fn pick_torrent_file() -> Result<String> {
    // zenity
    if cmd_exists("zenity") {
        let out = std::process::Command::new("zenity")
            .args([
                "--file-selection",
                "--title=Select torrent file",
                "--file-filter=Torrent files | *.torrent",
                "--file-filter=All files | *",
            ])
            .output()?;
        if out.status.success() {
            return parse_output(&out.stdout);
        }
        return Err(anyhow!("Cancelled"));
    }

    // kdialog
    if cmd_exists("kdialog") {
        let out = std::process::Command::new("kdialog")
            .args([
                "--getopenfilename", ".",
                "*.torrent|Torrent files\n*|All files",
                "--title", "Select torrent file",
            ])
            .output()?;
        if out.status.success() {
            return parse_output(&out.stdout);
        }
        return Err(anyhow!("Cancelled"));
    }

    // yad (Yet Another Dialog)
    if cmd_exists("yad") {
        let out = std::process::Command::new("yad")
            .args([
                "--file-selection",
                "--title=Select torrent file",
                "--file-filter=*.torrent",
            ])
            .output()?;
        if out.status.success() {
            return parse_output(&out.stdout);
        }
        return Err(anyhow!("Cancelled"));
    }

    Err(anyhow!("No file dialog found (install zenity or kdialog)"))
}

/// Открывает диалог выбора папки или файла для создания торрента.
pub fn pick_directory() -> Result<String> {
    if cmd_exists("zenity") {
        let out = std::process::Command::new("zenity")
            .args([
                "--file-selection",
                "--directory",
                "--title=Select folder to create torrent from",
            ])
            .output()?;
        if out.status.success() {
            return parse_output(&out.stdout);
        }
        // Если отменили directory mode — пробуем выбор файла
        let out = std::process::Command::new("zenity")
            .args([
                "--file-selection",
                "--title=Select file to create torrent from",
            ])
            .output()?;
        if out.status.success() {
            return parse_output(&out.stdout);
        }
        return Err(anyhow!("Cancelled"));
    }

    if cmd_exists("kdialog") {
        let out = std::process::Command::new("kdialog")
            .args(["--getexistingdirectory", ".", "--title", "Select folder"])
            .output()?;
        if out.status.success() {
            return parse_output(&out.stdout);
        }
        return Err(anyhow!("Cancelled"));
    }

    if cmd_exists("yad") {
        let out = std::process::Command::new("yad")
            .args(["--file-selection", "--directory", "--title=Select folder"])
            .output()?;
        if out.status.success() {
            return parse_output(&out.stdout);
        }
        return Err(anyhow!("Cancelled"));
    }

    Err(anyhow!("No file dialog found (install zenity or kdialog)"))
}

fn parse_output(bytes: &[u8]) -> Result<String> {
    let path = String::from_utf8_lossy(bytes).trim().to_string();
    if path.is_empty() {
        Err(anyhow!("Empty path returned"))
    } else {
        Ok(path)
    }
}

fn cmd_exists(cmd: &str) -> bool {
    std::process::Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
