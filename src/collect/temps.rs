//! Temperatures straight from hwmon, labelled with their chip ("k10temp Tctl",
//! "nvme Composite") so it is clear which device each reading belongs to.

use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Temp {
    pub label: String,
    pub celsius: f32,
}

/// Reads every `tempN_input` under `hwmon_root` (normally `/sys/class/hwmon`).
pub fn read_temps(hwmon_root: &Path) -> Vec<Temp> {
    let Ok(chips) = fs::read_dir(hwmon_root) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for chip in chips.flatten() {
        let dir = chip.path();
        let chip_name = fs::read_to_string(dir.join("name"))
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        let Ok(files) = fs::read_dir(&dir) else {
            continue;
        };
        let mut inputs: Vec<String> = files
            .flatten()
            .filter_map(|f| {
                let n = f.file_name().to_string_lossy().into_owned();
                (n.starts_with("temp") && n.ends_with("_input")).then_some(n)
            })
            .collect();
        inputs.sort();
        for input in inputs {
            let Some(milli) = fs::read_to_string(dir.join(&input))
                .ok()
                .and_then(|s| s.trim().parse::<i64>().ok())
            else {
                continue;
            };
            let celsius = milli as f32 / 1000.0;
            if celsius <= 0.0 || celsius > 150.0 {
                continue;
            }
            let base = input.trim_end_matches("_input");
            let sensor = fs::read_to_string(dir.join(format!("{base}_label")))
                .map(|s| s.trim().to_string())
                .unwrap_or_default();
            let label = match (chip_name.is_empty(), sensor.is_empty()) {
                (false, false) => format!("{chip_name} {sensor}"),
                (false, true) => chip_name.clone(),
                (true, false) => sensor,
                (true, true) => base.to_string(),
            };
            out.push(Temp { label, celsius });
        }
    }
    out.sort_by(|a, b| a.label.cmp(&b.label));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_with_chip_and_skips_bad_values() {
        let root = tempfile::tempdir().unwrap();
        let k10 = root.path().join("hwmon0");
        fs::create_dir_all(&k10).unwrap();
        fs::write(k10.join("name"), "k10temp\n").unwrap();
        fs::write(k10.join("temp1_input"), "52250\n").unwrap();
        fs::write(k10.join("temp1_label"), "Tctl\n").unwrap();
        fs::write(k10.join("temp2_input"), "-273000\n").unwrap();
        let nic = root.path().join("hwmon1");
        fs::create_dir_all(&nic).unwrap();
        fs::write(nic.join("name"), "r8169\n").unwrap();
        fs::write(nic.join("temp1_input"), "39000\n").unwrap();

        assert_eq!(
            read_temps(root.path()),
            vec![
                Temp {
                    label: "k10temp Tctl".into(),
                    celsius: 52.25
                },
                Temp {
                    label: "r8169".into(),
                    celsius: 39.0
                },
            ]
        );
    }
}
