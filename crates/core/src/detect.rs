use std::fmt;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vendor {
    Acer,
    Lenovo,
    Dell,
    Hp,
    Unknown,
}

impl Vendor {
    pub fn parse(text: &str) -> Self {
        let lower = text.trim().to_lowercase();
        if lower.is_empty() {
            return Self::Unknown;
        }
        if lower.contains("acer") {
            Self::Acer
        } else if lower.contains("lenovo") {
            Self::Lenovo
        } else if lower.contains("dell") {
            Self::Dell
        } else if lower == "hp" || lower.contains("hewlett") || lower.contains("hp ") {
            Self::Hp
        } else {
            Self::Unknown
        }
    }
}

impl fmt::Display for Vendor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Acer => "acer",
            Self::Lenovo => "lenovo",
            Self::Dell => "dell",
            Self::Hp => "hp",
            Self::Unknown => "unknown",
        };
        f.write_str(s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Bgrt,
}

impl fmt::Display for Backend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bgrt => f.write_str("bgrt"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Detection {
    pub vendor: Vendor,
    pub model: String,
    pub backend: Backend,
}

pub fn detect(dmi_dir: &Path) -> Detection {
    let vendor_text = fs::read_to_string(dmi_dir.join("sys_vendor"))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            fs::read_to_string(dmi_dir.join("board_vendor"))
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        })
        .unwrap_or_default();
    let vendor = Vendor::parse(&vendor_text);
    let model_raw = fs::read_to_string(dmi_dir.join("product_name"))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| String::from("unknown"));
    Detection {
        vendor,
        model: model_raw,
        backend: Backend::Bgrt,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn temp_dir(name: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("blf-detect-{}-{}", std::process::id(), name));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn write(d: &Path, name: &str, content: &str) {
        fs::write(d.join(name), content).unwrap();
    }

    #[test]
    fn acer() {
        let d = temp_dir("t1");
        write(&d, "sys_vendor", "Acer");
        write(&d, "product_name", "Example Model");
        let det = detect(&d);
        assert_eq!(det.vendor, Vendor::Acer);
        assert_eq!(det.model, "Example Model");
    }

    #[test]
    fn lenovo_odd() {
        let d = temp_dir("t2");
        write(&d, "sys_vendor", "LENOVO");
        let det = detect(&d);
        assert_eq!(det.vendor, Vendor::Lenovo);
    }

    #[test]
    fn dell_odd() {
        let d = temp_dir("t3");
        write(&d, "sys_vendor", "Dell Inc.");
        let det = detect(&d);
        assert_eq!(det.vendor, Vendor::Dell);
    }

    #[test]
    fn hp_strings() {
        for s in ["HP", "Hewlett-Packard"] {
            let d = temp_dir("t4");
            write(&d, "sys_vendor", s);
            let det = detect(&d);
            assert_eq!(det.vendor, Vendor::Hp, "{}", s);
        }
    }

    #[test]
    fn missing_files() {
        let d = temp_dir("t5");
        let det = detect(&d);
        assert_eq!(det.vendor, Vendor::Unknown);
        assert_eq!(det.model, "unknown");
    }

    #[test]
    fn empty_files() {
        let d = temp_dir("t6");
        write(&d, "sys_vendor", "   \n");
        write(&d, "product_name", "");
        let det = detect(&d);
        assert_eq!(det.vendor, Vendor::Unknown);
        assert_eq!(det.model, "unknown");
    }

    #[test]
    fn board_vendor_fallback() {
        let d = temp_dir("t7");
        write(&d, "board_vendor", "Acer");
        let det = detect(&d);
        assert_eq!(det.vendor, Vendor::Acer);
    }
}
