//! Cross-platform user background service installer (`playmat-worker install-service` / `uninstall-service`).
//!
//! Installs a user-scoped background worker service without requiring root/Administrator:
//! - macOS: `~/Library/LaunchAgents/studio.deimos.mana-worker.plist` (`launchctl`)
//! - Linux: `~/.config/systemd/user/mana-worker.service` (`systemctl --user`)
//! - Windows: per-user Scheduled Task `ManaAtHomeWorker` (`schtasks`)

use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Debug)]
pub struct ServiceOpts {
    pub exe: PathBuf,
    pub coordinator: String,
    pub name: String,
    pub token_file: String,
    pub jobs: usize,
    pub work: String,
    pub repo: String,
    pub idle: bool,
    pub gpu: Option<String>,
    pub auto_update: bool,
    pub with_ui: bool,
    pub ui_port: u16,
}

impl ServiceOpts {
    pub fn worker_args(&self) -> Vec<String> {
        let mut args = vec![
            if self.with_ui { "ui".into() } else { "run".into() },
            "--coordinator".into(),
            self.coordinator.clone(),
            "--name".into(),
            self.name.clone(),
            "--token-file".into(),
            self.token_file.clone(),
            "--jobs".into(),
            self.jobs.to_string(),
            "--work".into(),
            self.work.clone(),
            "--repo".into(),
            self.repo.clone(),
            "--when".into(),
            if self.idle { "idle".into() } else { "always".into() },
        ];
        if let Some(g) = &self.gpu {
            args.push("--gpu".into());
            args.push(g.clone());
        }
        if self.auto_update {
            args.push("--auto-update".into());
        }
        if self.with_ui {
            args.push("--port".into());
            args.push(self.ui_port.to_string());
        }
        args
    }
}

#[allow(dead_code)]
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

#[allow(dead_code)]
pub fn render_launchd_plist(opts: &ServiceOpts, log_path: &Path) -> String {
    let mut program_args = String::new();
    program_args.push_str(&format!("    <string>{}</string>\n", xml_escape(&opts.exe.display().to_string())));
    for arg in opts.worker_args() {
        program_args.push_str(&format!("    <string>{}</string>\n", xml_escape(&arg)));
    }
    let log = xml_escape(&log_path.display().to_string());
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>studio.deimos.mana-worker</string>
  <key>ProgramArguments</key>
  <array>
{program_args}  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
  <key>ThrottleInterval</key>
  <integer>15</integer>
  <key>StandardOutPath</key>
  <string>{log}</string>
  <key>StandardErrorPath</key>
  <string>{log}</string>
</dict>
</plist>
"#
    )
}

#[allow(dead_code)]
pub fn render_systemd_unit(opts: &ServiceOpts) -> String {
    let quote = |s: &str| {
        if s.chars().any(|c| c.is_whitespace() || c == '"') {
            format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
        } else {
            s.to_string()
        }
    };
    let mut cmd = vec![quote(&opts.exe.display().to_string())];
    cmd.extend(opts.worker_args().into_iter().map(|a| quote(&a)));
    format!(
        "[Unit]\n\
         Description=Mana@Home Volunteer Worker\n\
         After=network-online.target\n\
         Wants=network-online.target\n\n\
         [Service]\n\
         Type=simple\n\
         ExecStart={}\n\
         Restart=always\n\
         RestartSec=10\n\
         Nice=10\n\n\
         [Install]\n\
         WantedBy=default.target\n",
        cmd.join(" ")
    )
}

#[allow(dead_code)]
pub fn render_windows_task_tr(opts: &ServiceOpts) -> String {
    let quote = |s: &str| format!("\"{}\"", s.replace('"', "\\\""));
    let mut parts = vec![quote(&opts.exe.display().to_string())];
    parts.extend(opts.worker_args().into_iter().map(|a| if a.contains(' ') { quote(&a) } else { a }));
    parts.join(" ")
}

#[cfg(target_os = "macos")]
fn home_dir() -> Result<PathBuf, String> {
    std::env::var("HOME").map(PathBuf::from).map_err(|_| "HOME is not set".into())
}

#[cfg(target_os = "linux")]
fn home_dir() -> Result<PathBuf, String> {
    std::env::var("HOME").map(PathBuf::from).map_err(|_| "HOME is not set".into())
}

pub fn install(opts: &ServiceOpts) -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        let home = home_dir()?;
        let agents = home.join("Library/LaunchAgents");
        let logs = home.join("Library/Logs");
        std::fs::create_dir_all(&agents).map_err(|e| format!("mkdir {}: {e}", agents.display()))?;
        std::fs::create_dir_all(&logs).map_err(|e| format!("mkdir {}: {e}", logs.display()))?;
        let plist = agents.join("studio.deimos.mana-worker.plist");
        let log_path = logs.join("mana-worker.log");
        std::fs::write(&plist, render_launchd_plist(opts, &log_path))
            .map_err(|e| format!("write {}: {e}", plist.display()))?;
        let uid = unsafe { libc::getuid() };
        let domain = format!("gui/{uid}");
        let _ = Command::new("launchctl").args(["bootout", &domain, &plist.display().to_string()]).status();
        let st = Command::new("launchctl")
            .args(["bootstrap", &domain, &plist.display().to_string()])
            .status()
            .map_err(|e| format!("launchctl bootstrap: {e}"))?;
        if !st.success() {
            return Err(format!("launchctl bootstrap failed with {st}; plist saved at {}", plist.display()));
        }
        Ok(format!("installed macOS LaunchAgent {} (logs: {})", plist.display(), log_path.display()))
    }
    #[cfg(target_os = "linux")]
    {
        let home = home_dir()?;
        let dir = home.join(".config/systemd/user");
        std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir {}: {e}", dir.display()))?;
        let unit = dir.join("mana-worker.service");
        std::fs::write(&unit, render_systemd_unit(opts)).map_err(|e| format!("write {}: {e}", unit.display()))?;
        let _ = Command::new("systemctl").args(["--user", "daemon-reload"]).status();
        let st = Command::new("systemctl")
            .args(["--user", "enable", "--now", "mana-worker.service"])
            .status()
            .map_err(|e| format!("systemctl --user: {e}"))?;
        if !st.success() {
            return Err(format!("systemctl enable --now failed ({st}); unit saved at {}", unit.display()));
        }
        Ok(format!("installed systemd user service {}", unit.display()))
    }
    #[cfg(target_os = "windows")]
    {
        let tr = render_windows_task_tr(opts);
        let st = Command::new("schtasks")
            .args(["/Create", "/F", "/SC", "ONLOGON", "/TN", "ManaAtHomeWorker", "/TR", &tr])
            .status()
            .map_err(|e| format!("schtasks /Create: {e}"))?;
        if !st.success() {
            return Err(format!("schtasks /Create failed ({st})"));
        }
        let _ = Command::new("schtasks").args(["/Run", "/TN", "ManaAtHomeWorker"]).status();
        Ok("installed and started Windows Scheduled Task ManaAtHomeWorker".into())
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = opts;
        Err("install-service is supported on macOS, Linux, and Windows".into())
    }
}

pub fn uninstall() -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        let plist = home_dir()?.join("Library/LaunchAgents/studio.deimos.mana-worker.plist");
        let uid = unsafe { libc::getuid() };
        let _ =
            Command::new("launchctl").args(["bootout", &format!("gui/{uid}"), &plist.display().to_string()]).status();
        let _ = std::fs::remove_file(&plist);
        Ok(format!("removed macOS LaunchAgent {}", plist.display()))
    }
    #[cfg(target_os = "linux")]
    {
        let unit = home_dir()?.join(".config/systemd/user/mana-worker.service");
        let _ = Command::new("systemctl").args(["--user", "disable", "--now", "mana-worker.service"]).status();
        let _ = std::fs::remove_file(&unit);
        let _ = Command::new("systemctl").args(["--user", "daemon-reload"]).status();
        Ok(format!("removed systemd user service {}", unit.display()))
    }
    #[cfg(target_os = "windows")]
    {
        let _ = Command::new("schtasks").args(["/End", "/TN", "ManaAtHomeWorker"]).status();
        let _ = Command::new("schtasks").args(["/Delete", "/F", "/TN", "ManaAtHomeWorker"]).status();
        Ok("removed Windows Scheduled Task ManaAtHomeWorker".into())
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        Err("uninstall-service is supported on macOS, Linux, and Windows".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_opts() -> ServiceOpts {
        ServiceOpts {
            exe: PathBuf::from("/usr/local/bin/playmat-worker"),
            coordinator: "https://mana.deimos.studio".into(),
            name: "my-box".into(),
            token_file: "/home/u/.config/mana/worker.token".into(),
            jobs: 4,
            work: "/home/u/.cache/mana".into(),
            repo: "/opt/mana".into(),
            idle: true,
            gpu: Some("auto".into()),
            auto_update: true,
            with_ui: true,
            ui_port: 17195,
        }
    }

    #[test]
    fn renders_launchd_systemd_and_windows_configs() {
        let o = sample_opts();
        let plist = render_launchd_plist(&o, Path::new("/tmp/mana.log"));
        assert!(plist.contains("<string>studio.deimos.mana-worker</string>"));
        assert!(plist.contains("<string>ui</string>"));
        assert!(plist.contains("<string>--auto-update</string>"));
        assert!(plist.contains("<string>auto</string>"));

        let unit = render_systemd_unit(&o);
        assert!(unit.contains("ExecStart=/usr/local/bin/playmat-worker ui --coordinator https://mana.deimos.studio"));
        assert!(unit.contains("--auto-update"));

        let tr = render_windows_task_tr(&o);
        assert!(tr.contains("\"/usr/local/bin/playmat-worker\" ui"));
        assert!(tr.contains("--port 17195"));
    }
}
