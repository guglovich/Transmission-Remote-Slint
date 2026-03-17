// src/disks.rs
// Маппинг пути → физический диск через stat()+major:minor+lsblk.
// Работает с любой топологией: BTRFS, LVM, RAID, bind mounts.

use std::collections::HashMap;
use std::process::Command;
use serde::Deserialize;

/// Физический диск (только реальные /dev/sda, /dev/nvme0n1 и т.п.)
#[derive(Debug, Clone)]
pub struct PhysicalDisk {
    /// /dev/sda, /dev/nvme0n1
    pub dev:   String,
    /// Модель: "Samsung SSD 870" или имя устройства если модель неизвестна
    pub label: String,
}

#[derive(Debug, Deserialize)]
struct LsblkRoot { blockdevices: Vec<LsblkDev> }

#[derive(Debug, Deserialize, Clone)]
struct LsblkDev {
    name:     String,
    #[serde(rename = "type")]
    kind:     String,
    #[serde(rename = "maj:min", default)]
    majmin:   String,
    #[serde(default)]
    model:    Option<String>,
    #[serde(default)]
    children: Vec<LsblkDev>,
}

pub fn detect_physical_disks() -> Vec<PhysicalDisk> {
    let Ok(out) = Command::new("lsblk")
        .args(["--json", "-o", "NAME,TYPE,MAJ:MIN,MODEL"])
        .output() else {
        eprintln!("[disks] lsblk failed");
        return vec![];
    };
    let Ok(root) = serde_json::from_slice::<LsblkRoot>(&out.stdout) else {
        eprintln!("[disks] lsblk parse failed");
        return vec![];
    };
    let mut disks = vec![];
    for dev in &root.blockdevices {
        // Исключаем виртуальные устройства
        let name = &dev.name;
        if name.starts_with("zram") || name.starts_with("loop") ||
           name.starts_with("ram")  || name.starts_with("dm-")  ||
           dev.kind != "disk" {
            continue;
        }
        let label = dev.model.clone()
            .map(|m| m.trim().to_string())
            .filter(|m| !m.is_empty())
            .unwrap_or_else(|| dev.name.clone());
        disks.push(PhysicalDisk { dev: format!("/dev/{}", dev.name), label });
    }
    eprintln!("[disks] Physical disks: {:?}", disks.iter().map(|d| &d.dev).collect::<Vec<_>>());
    disks
}

/// maj:min → /dev/sdX (физический родитель)
pub fn build_majmin_map() -> HashMap<(u32, u32), String> {
    let Ok(out) = Command::new("lsblk")
        .args(["--json", "-o", "NAME,TYPE,MAJ:MIN"])
        .output() else {
        eprintln!("[disks] lsblk for majmin failed");
        return HashMap::new()
    };
    eprintln!("[disks] lsblk majmin output: {} bytes", out.stdout.len());
    let Ok(root) = serde_json::from_slice::<LsblkRoot>(&out.stdout) else {
        eprintln!("[disks] lsblk majmin parse failed");
        return HashMap::new()
    };
    let mut map = HashMap::new();
    fn walk(dev: &LsblkDev, disk: &str, map: &mut HashMap<(u32, u32), String>) {
        let d = if dev.kind == "disk" { &dev.name } else { disk };
        if let Some(mm) = parse_majmin(&dev.majmin) {
            map.insert(mm, format!("/dev/{d}"));
        }
        for c in &dev.children { walk(c, d, map); }
    }
    for dev in &root.blockdevices { walk(dev, &dev.name, &mut map); }
    map
}

fn parse_majmin(s: &str) -> Option<(u32, u32)> {
    let mut p = s.splitn(2, ':');
    Some((p.next()?.parse().ok()?, p.next()?.parse().ok()?))
}

/// stat(path) → /dev/sdX
pub fn disk_for_path(path: &str, map: &HashMap<(u32, u32), String>) -> Option<String> {
    use std::os::unix::fs::MetadataExt;
    let dev = std::fs::metadata(path).ok()?.dev();
    let maj = libc::major(dev) as u32;
    let min = libc::minor(dev) as u32;
    map.get(&(maj, min)).cloned()
}
