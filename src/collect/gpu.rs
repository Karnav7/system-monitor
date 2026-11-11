//! AMD GPU readings from the amdgpu driver's sysfs files.

use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub struct GpuInfo {
    pub card: String,
    pub busy: Option<u8>,
    pub vram_used: Option<u64>,
    pub vram_total: Option<u64>,
}

fn read_num(path: &Path) -> Option<u64> {
    fs::read_to_string(path).ok()?.trim().parse().ok()
}

/// Reads every `cardN` under `drm_root` (normally `/sys/class/drm`) that
/// exposes `gpu_busy_percent`, which only amdgpu does. Connector entries such
/// as `card0-DP-1` are skipped.
pub fn read_gpus(drm_root: &Path) -> Vec<GpuInfo> {
    let Ok(entries) = fs::read_dir(drm_root) else {
        return Vec::new();
    };
    let mut gpus: Vec<GpuInfo> = entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let is_card = name
                .strip_prefix("card")
                .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()));
            if !is_card {
                return None;
            }
            let dev = e.path().join("device");
            let busy = read_num(&dev.join("gpu_busy_percent"))?;
            Some(GpuInfo {
                card: name,
                busy: Some(busy.min(100) as u8),
                vram_used: read_num(&dev.join("mem_info_vram_used")),
                vram_total: read_num(&dev.join("mem_info_vram_total")),
            })
        })
        .collect();
    gpus.sort_by(|a, b| a.card.cmp(&b.card));
    gpus
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(root: &Path, name: &str, files: &[(&str, &str)]) {
        let dev = root.join(name).join("device");
        fs::create_dir_all(&dev).unwrap();
        for (f, v) in files {
            fs::write(dev.join(f), v).unwrap();
        }
    }

    #[test]
    fn reads_amdgpu_and_skips_others() {
        let root = tempfile::tempdir().unwrap();
        card(
            root.path(),
            "card0",
            &[
                ("gpu_busy_percent", "42\n"),
                ("mem_info_vram_used", "1024\n"),
                ("mem_info_vram_total", "4096\n"),
            ],
        );
        // A non-amdgpu card (no gpu_busy_percent) and a connector entry.
        card(root.path(), "card1", &[("vendor", "0x1234")]);
        card(root.path(), "card0-DP-1", &[("gpu_busy_percent", "1")]);

        let gpus = read_gpus(root.path());
        assert_eq!(
            gpus,
            vec![GpuInfo {
                card: "card0".into(),
                busy: Some(42),
                vram_used: Some(1024),
                vram_total: Some(4096)
            }]
        );
    }

    #[test]
    fn missing_root_is_empty() {
        assert!(read_gpus(Path::new("/definitely/not/here")).is_empty());
    }
}
