// src/benchmark/system_info.rs

use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub hostname: String,
    pub os: String,
    pub os_version: String,
    pub cpu_model: String,
    pub cpu_cores: usize,
    pub cpu_threads: usize,
    pub total_memory_mb: u64,
    pub git_commit: String,
    pub git_branch: String,
    pub git_dirty: bool,
    pub rust_version: String,
}

impl SystemInfo {
    pub fn collect() -> Self {
        use sysinfo::System;

        let mut sys = System::new_all();
        sys.refresh_all();

        // CPU info
        let cpu_model = sys.cpus()
            .first()
            .map(|cpu| cpu.brand().to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        let cpu_cores = sys.physical_core_count().unwrap_or(0);
        let cpu_threads = sys.cpus().len();

        // Memory in MB
        let total_memory_mb = sys.total_memory() / 1024 / 1024;

        // OS info
        let os = System::name().unwrap_or_else(|| "Unknown".to_string());
        let os_version = System::os_version().unwrap_or_else(|| "Unknown".to_string());
        let hostname = System::host_name().unwrap_or_else(|| "Unknown".to_string());

        // Git info
        let (git_commit, git_branch, git_dirty) = Self::get_git_info();

        // Rust version
        let rust_version = Self::get_rust_version();

        SystemInfo {
            hostname,
            os,
            os_version,
            cpu_model,
            cpu_cores,
            cpu_threads,
            total_memory_mb,
            git_commit,
            git_branch,
            git_dirty,
            rust_version,
        }
    }

    fn get_git_info() -> (String, String, bool) {
        match git2::Repository::open(".") {
            Ok(repo) => {
                let head = repo.head().ok();

                let commit = head.as_ref()
                    .and_then(|h| h.peel_to_commit().ok())
                    .map(|c| c.id().to_string())
                    .unwrap_or_else(|| "unknown".to_string());

                let branch = head.as_ref()
                    .and_then(|h| h.shorthand())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "unknown".to_string());

                // Check if working directory is dirty
                let dirty = repo.statuses(None)
                    .map(|statuses| !statuses.is_empty())
                    .unwrap_or(false);

                (commit, branch, dirty)
            }
            Err(_) => ("unknown".to_string(), "unknown".to_string(), false)
        }
    }

    fn get_rust_version() -> String {
        env::var("RUSTC_VERSION")
            .unwrap_or_else(|_| {
                // Try to get from rustc --version
                std::process::Command::new("rustc")
                    .arg("--version")
                    .output()
                    .ok()
                    .and_then(|output| String::from_utf8(output.stdout).ok())
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|| "unknown".to_string())
            })
    }

    pub fn to_string_pretty(&self) -> String {
        format!(
            r#"System Information:
  Hostname:     {}
  OS:           {} {}
  CPU:          {} ({} cores, {} threads)
  Memory:       {} MB
  Git:          {} ({}){}
  Rust:         {}
"#,
            self.hostname,
            self.os,
            self.os_version,
            self.cpu_model,
            self.cpu_cores,
            self.cpu_threads,
            self.total_memory_mb,
            self.git_commit.chars().take(8).collect::<String>(),
            self.git_branch,
            if self.git_dirty { " [dirty]" } else { "" },
            self.rust_version,
        )
    }
}
