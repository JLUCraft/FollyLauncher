use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JavaRuntime {
    pub exec_path: String,
    pub major_version: i32,
    pub vendor: String,
}

pub async fn scan_java_runtimes() -> Vec<JavaRuntime> {
    let mut runtimes = Vec::new();

    // Scan PATH for java
    if let Ok(path) = which::which("java") {
        if let Some(rt) = detect_java_runtime(&path).await {
            runtimes.push(rt);
        }
    }
    #[cfg(target_os = "windows")]
    if let Ok(path) = which::which("javaw") {
        if let Some(rt) = detect_java_runtime(&path).await {
            if !runtimes.iter().any(|r| r.exec_path == rt.exec_path) {
                runtimes.push(rt);
            }
        }
    }

    // Scan common installation paths
    for path in get_common_java_paths() {
        if let Some(rt) = detect_java_runtime(&path).await {
            if !runtimes.iter().any(|r| r.exec_path == rt.exec_path) {
                runtimes.push(rt);
            }
        }
    }

    runtimes.sort_by_key(|r| r.major_version);
    runtimes
}

async fn detect_java_runtime(path: &Path) -> Option<JavaRuntime> {
    let output = Command::new(path).arg("-version").output().ok()?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    let (major_version, vendor) = parse_java_version(&stderr)?;

    Some(JavaRuntime {
        exec_path: path.to_string_lossy().to_string(),
        major_version,
        vendor,
    })
}

fn parse_java_version(output: &str) -> Option<(i32, String)> {
    let version_line = output.lines().next()?;
    // Match patterns like: openjdk version "17.0.8" 2023-07-18
    // or: java version "1.8.0_361"
    let version_str = version_line.split('"').nth(1)?;

    let major = if version_str.starts_with("1.") {
        // Java 8 and earlier: 1.8.0 -> 8
        version_str.split('.').nth(1)?.parse::<i32>().ok()?
    } else {
        version_str.split('.').next()?.parse::<i32>().ok()?
    };

    let vendor = if output.contains("OpenJDK") || output.contains("openjdk") {
        "OpenJDK"
    } else if output.contains("Oracle") || output.contains("Java(TM)") {
        "Oracle"
    } else if output.contains("Temurin") || output.contains("Adoptium") {
        "Eclipse Temurin"
    } else if output.contains("GraalVM") {
        "GraalVM"
    } else if output.contains("Zulu") || output.contains("Azul") {
        "Azul Zulu"
    } else {
        "Unknown"
    }
    .to_string();

    Some((major, vendor))
}

fn get_common_java_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    #[cfg(target_os = "windows")]
    {
        let prog_files = std::env::var("ProgramFiles").unwrap_or_default();
        let prog_files_x86 = std::env::var("ProgramFiles(x86)").unwrap_or_default();

        for base in [&prog_files, &prog_files_x86] {
            let java_dir = PathBuf::from(base).join("Java");
            if let Ok(entries) = std::fs::read_dir(&java_dir) {
                for entry in entries.flatten() {
                    let bin = entry.path().join("bin").join("java.exe");
                    if bin.exists() {
                        paths.push(bin);
                    }
                }
            }
        }

        // Microsoft Build of OpenJDK
        let local_appdata = std::env::var("LOCALAPPDATA").unwrap_or_default();
        let ms_java = PathBuf::from(&local_appdata).join("Microsoft").join("Java");
        if let Ok(entries) = std::fs::read_dir(&ms_java) {
            for entry in entries.flatten() {
                let bin = entry.path().join("bin").join("java.exe");
                if bin.exists() {
                    paths.push(bin);
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let jvms = PathBuf::from("/Library/Java/JavaVirtualMachines");
        if let Ok(entries) = std::fs::read_dir(&jvms) {
            for entry in entries.flatten() {
                let bin = entry
                    .path()
                    .join("Contents")
                    .join("Home")
                    .join("bin")
                    .join("java");
                if bin.exists() {
                    paths.push(bin);
                }
            }
        }
        if let Ok(home) = std::env::var("HOME") {
            let user_jvms = PathBuf::from(home)
                .join("Library")
                .join("Java")
                .join("JavaVirtualMachines");
            if let Ok(entries) = std::fs::read_dir(&user_jvms) {
                for entry in entries.flatten() {
                    let bin = entry
                        .path()
                        .join("Contents")
                        .join("Home")
                        .join("bin")
                        .join("java");
                    if bin.exists() {
                        paths.push(bin);
                    }
                }
            }
            let jdks = PathBuf::from(home).join(".jdks");
            if let Ok(entries) = std::fs::read_dir(&jdks) {
                for entry in entries.flatten() {
                    let bin = entry.path().join("bin").join("java");
                    if bin.exists() {
                        paths.push(bin);
                    }
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        let jvm = PathBuf::from("/usr/lib/jvm");
        if let Ok(entries) = std::fs::read_dir(&jvm) {
            for entry in entries.flatten() {
                let bin = entry.path().join("bin").join("java");
                if bin.exists() {
                    paths.push(bin);
                }
            }
        }
        if let Ok(home) = std::env::var("HOME") {
            let jdks = PathBuf::from(home).join(".jdks");
            if let Ok(entries) = std::fs::read_dir(&jdks) {
                for entry in entries.flatten() {
                    let bin = entry.path().join("bin").join("java");
                    if bin.exists() {
                        paths.push(bin);
                    }
                }
            }
        }
    }

    paths
}

pub fn get_minimum_java_version(game_version: &str) -> i32 {
    // 26.1+ (snapshots) -> Java 25
    if game_version.starts_with("26.") {
        return 25;
    }
    // 1.21+ / 1.20.5+ -> Java 21
    if game_version.starts_with("1.21")
        || game_version.starts_with("1.20.5")
        || game_version.starts_with("1.20.6")
        || game_version.starts_with("1.22")
    {
        return 21;
    }
    // 1.20-1.20.4 / 1.19.x -> Java 17
    if game_version.starts_with("1.20") || game_version.starts_with("1.19") {
        return 17;
    }
    // 1.18 -> Java 17
    if game_version.starts_with("1.18") {
        return 17;
    }
    // 1.17 -> Java 16
    if game_version.starts_with("1.17") {
        return 16;
    }
    // 1.12-1.16 -> Java 8
    if game_version.starts_with("1.12")
        || game_version.starts_with("1.13")
        || game_version.starts_with("1.14")
        || game_version.starts_with("1.15")
        || game_version.starts_with("1.16")
    {
        return 8;
    }
    // Older versions
    8
}

pub fn select_java_runtime(
    runtimes: &[JavaRuntime],
    game_version: &str,
    preferred_path: Option<&str>,
    client_json_major: Option<i32>,
) -> Option<JavaRuntime> {
    let min_version = if let Some(major) = client_json_major {
        // Priority: use `java_version.major_version` from client JSON
        major
    } else {
        get_minimum_java_version(game_version)
    };

    // Prefer exact match first
    if let Some(rt) = runtimes.iter().find(|r| r.major_version == min_version) {
        return Some(rt.clone());
    }

    // Then preferred path if it meets minimum
    if let Some(path) = preferred_path {
        if let Some(rt) = runtimes
            .iter()
            .find(|r| r.exec_path == path && r.major_version >= min_version)
        {
            return Some(rt.clone());
        }
    }

    // Lowest suitable version
    runtimes
        .iter()
        .filter(|r| r.major_version >= min_version)
        .min_by_key(|r| r.major_version)
        .cloned()
}

pub async fn validate_java_path(java_path: &str) -> Option<JavaRuntime> {
    detect_java_runtime(Path::new(java_path)).await
}
