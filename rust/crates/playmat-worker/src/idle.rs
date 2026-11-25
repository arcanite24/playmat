//! When a contributor's machine counts as idle (docs/swarm/Design.md, "Worker platforms"), ported from the LAN
//! fleet's rules (`rust/rl/fleet.py`): a Mac with no keyboard or mouse input for 10 minutes and a low load; Windows
//! on AC power with other processes under 40% of the CPU; Linux on mains power, no Steam game running, and a low load.
//! While a unit plays, the worker's own games don't count, and a Mac only needs a minute without input.

use std::process::Command;

const IDLE_MAC_S: u64 = 600;
const LOAD_MAX: f64 = 0.35;
const CPU_OTHER_MAX: f64 = 40.0;

fn sh(cmd: &str, args: &[&str]) -> String {
    Command::new(cmd).args(args).output().map(|o| String::from_utf8_lossy(&o.stdout).to_string()).unwrap_or_default()
}

/// Why the machine isn't idle, or `None`
pub fn busy(playing: bool) -> Option<String> {
    if cfg!(target_os = "macos") {
        let idle_s = sh("ioreg", &["-c", "IOHIDSystem"])
            .lines()
            .find(|l| l.contains("HIDIdleTime"))
            .and_then(|l| l.rsplit(' ').next()?.trim().parse::<u64>().ok())
            .map(|ns| ns / 1_000_000_000)
            .unwrap_or(u64::MAX);
        if idle_s < if playing { 60 } else { IDLE_MAC_S } {
            return Some(format!("in use ({idle_s} s since the last input)"));
        }
        let load: f64 =
            sh("sysctl", &["-n", "vm.loadavg"]).split_whitespace().nth(1).and_then(|v| v.parse().ok()).unwrap_or(0.0);
        let cores = std::thread::available_parallelism().map_or(1, |n| n.get()) as f64;
        if !playing && load / cores > LOAD_MAX {
            return Some(format!("load {load:.1} on {cores} cores"));
        }
        return None;
    }
    if cfg!(windows) {
        let out = sh(
            "powershell",
            &[
                "-NoProfile",
                "-Command",
                "$b = Get-CimInstance Win32_Battery -ErrorAction SilentlyContinue | Select-Object -First 1; \
                 $ac = if ($b) { $b.BatteryStatus -eq 2 } else { $true }; \
                 $n = (Get-CimInstance Win32_ComputerSystem).NumberOfLogicalProcessors; \
                 $s = Get-CimInstance Win32_PerfFormattedData_PerfProc_Process | \
                 Where-Object { $_.Name -notmatch '^(_Total|Idle|playmat-worker.*|selfplay.*)$' } | \
                 Measure-Object PercentProcessorTime -Sum; \
                 '{0} {1}' -f $ac, [int]($s.Sum / $n)",
            ],
        );
        let mut it = out.split_whitespace();
        if it.next() == Some("False") {
            return Some("on battery".into());
        }
        let other: f64 = it.next().and_then(|v| v.parse().ok()).unwrap_or(0.0);
        if other > CPU_OTHER_MAX {
            return Some(format!("other processes use {other}% CPU"));
        }
        return None;
    }
    // Linux: mains power, no Steam game (Steam starts every game under `reaper SteamLaunch`), a low load
    let on_battery = std::fs::read_dir("/sys/class/power_supply").ok().is_some_and(|dir| {
        dir.flatten().any(|e| {
            e.file_name().to_string_lossy().starts_with("AC")
                && std::fs::read_to_string(e.path().join("online")).is_ok_and(|v| v.trim() == "0")
        })
    });
    if on_battery {
        return Some("on battery".into());
    }
    if Command::new("pgrep").args(["-f", "[r]eaper SteamLaunch"]).status().is_ok_and(|s| s.success()) {
        return Some("a Steam game is running".into());
    }
    let load: f64 = std::fs::read_to_string("/proc/loadavg")
        .ok()
        .and_then(|v| v.split_whitespace().next()?.parse().ok())
        .unwrap_or(0.0);
    let cores = std::thread::available_parallelism().map_or(1, |n| n.get()) as f64;
    if !playing && load / cores > LOAD_MAX {
        return Some(format!("load {load:.1} on {cores} cores"));
    }
    None
}

/// Runs the worker below other programs: nice 15 on Unix, below-normal priority on Windows
pub fn lower_priority() {
    #[cfg(unix)]
    unsafe {
        libc::setpriority(libc::PRIO_PROCESS, 0, 15);
    }
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::System::Threading::{GetCurrentProcess, SetPriorityClass, BELOW_NORMAL_PRIORITY_CLASS};
        SetPriorityClass(GetCurrentProcess(), BELOW_NORMAL_PRIORITY_CLASS);
    }
}
