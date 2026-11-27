//! Local desktop worker control panel (`playmat-worker ui`).
//!
//! Binds strictly to `127.0.0.1:<port>` (default 17195) using `std::net::TcpListener` so desktop volunteers can
//! inspect status, pause/resume, adjust CPU thread count, toggle idle-only mode, and read live logs in a browser
//! without needing an Electron/Tauri runtime.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

const MAX_LOGS: usize = 60;

#[derive(Clone, Debug, Serialize)]
pub struct UiSnapshot {
    pub name: String,
    pub coordinator: String,
    pub class: String,
    pub status: String,
    pub current_unit: Option<String>,
    pub paused: bool,
    pub idle_only: bool,
    pub jobs: usize,
    pub max_jobs: usize,
    pub units_done: usize,
    pub games_done: usize,
    pub logs: Vec<String>,
}

pub struct Control {
    pub paused: AtomicBool,
    pub idle_only: AtomicBool,
    pub jobs: AtomicUsize,
    max_jobs: usize,
    inner: Mutex<Inner>,
}

struct Inner {
    name: String,
    coordinator: String,
    class: String,
    status: String,
    current_unit: Option<String>,
    units_done: usize,
    games_done: usize,
    logs: VecDeque<String>,
}

impl Control {
    pub fn new(name: String, coordinator: String, class: String, jobs: usize, idle_only: bool) -> Arc<Self> {
        let max_jobs = std::thread::available_parallelism().map_or(jobs.max(1), |n| n.get());
        Arc::new(Self {
            paused: AtomicBool::new(false),
            idle_only: AtomicBool::new(idle_only),
            jobs: AtomicUsize::new(jobs.clamp(1, max_jobs)),
            max_jobs,
            inner: Mutex::new(Inner {
                name,
                coordinator,
                class,
                status: "starting".into(),
                current_unit: None,
                units_done: 0,
                games_done: 0,
                logs: VecDeque::with_capacity(MAX_LOGS),
            }),
        })
    }

    pub fn set_status(&self, status: &str, unit: Option<String>) {
        let mut g = self.inner.lock().unwrap();
        g.status = status.into();
        g.current_unit = unit;
    }

    pub fn record_done(&self, games: usize) {
        let mut g = self.inner.lock().unwrap();
        g.units_done += 1;
        g.games_done += games;
        g.current_unit = None;
    }

    pub fn log(&self, line: impl Into<String>) {
        let mut g = self.inner.lock().unwrap();
        if g.logs.len() >= MAX_LOGS {
            g.logs.pop_front();
        }
        g.logs.push_back(line.into());
    }

    pub fn snapshot(&self) -> UiSnapshot {
        let g = self.inner.lock().unwrap();
        UiSnapshot {
            name: g.name.clone(),
            coordinator: g.coordinator.clone(),
            class: g.class.clone(),
            status: g.status.clone(),
            current_unit: g.current_unit.clone(),
            paused: self.paused.load(Ordering::Relaxed),
            idle_only: self.idle_only.load(Ordering::Relaxed),
            jobs: self.jobs.load(Ordering::Relaxed),
            max_jobs: self.max_jobs,
            units_done: g.units_done,
            games_done: g.games_done,
            logs: g.logs.iter().cloned().collect(),
        }
    }
}

#[derive(Deserialize)]
struct ConfigUpdate {
    jobs: Option<usize>,
    idle_only: Option<bool>,
}

/// Starts the loopback control HTTP server on `127.0.0.1:<port>` in a background thread and returns the bound address.
pub fn spawn_server(port: u16, control: Arc<Control>) -> Result<String, String> {
    let listener = TcpListener::bind(("127.0.0.1", port)).map_err(|e| format!("bind 127.0.0.1:{port}: {e}"))?;
    let addr = listener.local_addr().map_err(|e| e.to_string())?.to_string();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let c = control.clone();
            std::thread::spawn(move || {
                let _ = handle_conn(stream, &c);
            });
        }
    });
    Ok(addr)
}

fn handle_conn(mut stream: TcpStream, control: &Control) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut req_line = String::new();
    if reader.read_line(&mut req_line)? == 0 {
        return Ok(());
    }
    let mut parts = req_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("/");

    let mut content_len = 0usize;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line)?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            break;
        }
        if let Some((k, v)) = trimmed.split_once(':') {
            if k.eq_ignore_ascii_case("content-length") {
                content_len = v.trim().parse().unwrap_or(0).min(16 * 1024);
            }
        }
    }
    let mut body = vec![0u8; content_len];
    if content_len > 0 {
        reader.read_exact(&mut body)?;
    }

    let (status, ctype, resp) = dispatch(method, path, &body, control);
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        resp.len()
    )?;
    stream.write_all(&resp)?;
    stream.flush()
}

fn dispatch(method: &str, path: &str, body: &[u8], control: &Control) -> (&'static str, &'static str, Vec<u8>) {
    match (method, path) {
        ("GET", "/") => ("200 OK", "text/html; charset=utf-8", DASHBOARD_HTML.as_bytes().to_vec()),
        ("GET", "/api/state") => {
            ("200 OK", "application/json", serde_json::to_vec(&control.snapshot()).unwrap_or_default())
        }
        ("POST", "/api/pause") => {
            control.paused.store(true, Ordering::Relaxed);
            control.set_status("paused", None);
            control.log("UI: paused by user");
            ("200 OK", "application/json", serde_json::to_vec(&control.snapshot()).unwrap_or_default())
        }
        ("POST", "/api/resume") => {
            control.paused.store(false, Ordering::Relaxed);
            control.set_status("running", None);
            control.log("UI: resumed by user");
            ("200 OK", "application/json", serde_json::to_vec(&control.snapshot()).unwrap_or_default())
        }
        ("POST", "/api/config") => match serde_json::from_slice::<ConfigUpdate>(body) {
            Ok(upd) => {
                if let Some(j) = upd.jobs {
                    let clamped = j.clamp(1, control.max_jobs);
                    control.jobs.store(clamped, Ordering::Relaxed);
                    control.log(format!("UI: CPU jobs set to {clamped}"));
                }
                if let Some(idle) = upd.idle_only {
                    control.idle_only.store(idle, Ordering::Relaxed);
                    control.log(format!("UI: idle-only mode = {idle}"));
                }
                ("200 OK", "application/json", serde_json::to_vec(&control.snapshot()).unwrap_or_default())
            }
            Err(e) => ("400 Bad Request", "application/json", format!("{{\"error\":\"{e}\"}}").into_bytes()),
        },
        _ => ("404 Not Found", "text/plain", b"not found".to_vec()),
    }
}

const DASHBOARD_HTML: &str = r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>Mana@Home Worker Control</title>
<style>
  :root { --bg:#13161b; --card:#1b2028; --ink:#ebe6db; --dim:#98a0ad; --accent:#d8a24a; --ok:#6ea87d; --border:#2b323e; }
  body { margin:0; font:14px/1.5 system-ui,-apple-system,sans-serif; background:var(--bg); color:var(--ink); padding:24px; }
  main { max-width:760px; margin:0 auto; display:grid; gap:16px; }
  .card { background:var(--card); border:1px solid var(--border); border-radius:10px; padding:16px; display:grid; gap:12px; }
  .row { display:flex; flex-wrap:wrap; gap:12px; align-items:center; justify-content:space-between; }
  h1,h2 { margin:0; text-transform:uppercase; letter-spacing:0.06em; font-size:16px; }
  .kpi { display:grid; grid-template-columns:repeat(auto-fit,minmax(140px,1fr)); gap:10px; }
  .kpi div { background:var(--bg); border:1px solid var(--border); border-radius:8px; padding:10px; }
  .kpi small { display:block; color:var(--dim); font-size:11px; text-transform:uppercase; }
  .kpi strong { font-size:20px; font-variant-numeric:tabular-nums; }
  button { background:var(--accent); color:#13161b; border:0; border-radius:6px; padding:8px 14px; font-weight:700; cursor:pointer; }
  button.sec { background:#2b323e; color:var(--ink); }
  label { display:inline-flex; align-items:center; gap:8px; color:var(--dim); }
  input[type=number] { width:64px; background:var(--bg); color:var(--ink); border:1px solid var(--border); border-radius:6px; padding:6px; }
  pre { margin:0; background:var(--bg); border:1px solid var(--border); border-radius:8px; padding:12px; max-height:260px; overflow:auto; font:12px/1.45 ui-monospace,monospace; color:var(--dim); }
</style>
</head>
<body>
<main>
  <section class="card">
    <div class="row">
      <div><h1 id="title">Mana@Home Worker</h1><div id="sub" style="color:var(--dim);font-size:12px"></div></div>
      <div><button id="toggleBtn" type="button">Pause</button></div>
    </div>
    <div class="kpi">
      <div><small>Status</small><strong id="st">-</strong></div>
      <div><small>Current Unit</small><strong id="unit" style="font-size:14px">-</strong></div>
      <div><small>Units Done</small><strong id="units">0</strong></div>
      <div><small>Games Played</small><strong id="games">0</strong></div>
    </div>
  </section>
  <section class="card">
    <h2>Settings</h2>
    <div class="row">
      <label>CPU threads <input id="jobs" type="number" min="1" max="64"></label>
      <label><input id="idle" type="checkbox"> Only run when computer is idle</label>
      <button id="saveBtn" class="sec" type="button">Apply</button>
    </div>
  </section>
  <section class="card">
    <h2>Recent activity</h2>
    <pre id="logs">Waiting for worker...</pre>
  </section>
</main>
<script>
let paused = false, loaded = false;
async function refresh() {
  const s = await (await fetch('/api/state')).json();
  paused = s.paused;
  document.getElementById('title').textContent = 'Mana@Home · ' + s.name;
  document.getElementById('sub').textContent = s.coordinator + ' · ' + s.class;
  document.getElementById('st').textContent = s.paused ? 'paused' : s.status;
  document.getElementById('unit').textContent = s.current_unit || 'none';
  document.getElementById('units').textContent = s.units_done;
  document.getElementById('games').textContent = s.games_done;
  document.getElementById('toggleBtn').textContent = s.paused ? 'Resume' : 'Pause';
  if (!loaded) {
    const j = document.getElementById('jobs');
    j.max = s.max_jobs; j.value = s.jobs;
    document.getElementById('idle').checked = s.idle_only;
    loaded = true;
  }
  document.getElementById('logs').textContent = s.logs.length ? s.logs.join('\n') : 'No activity yet.';
}
document.getElementById('toggleBtn').onclick = async () => {
  await fetch(paused ? '/api/resume' : '/api/pause', { method: 'POST' });
  refresh();
};
document.getElementById('saveBtn').onclick = async () => {
  const jobs = Number(document.getElementById('jobs').value) || 1;
  const idle_only = document.getElementById('idle').checked;
  await fetch('/api/config', { method: 'POST', headers: {'Content-Type':'application/json'}, body: JSON.stringify({ jobs, idle_only }) });
  refresh();
};
refresh();
setInterval(refresh, 2000);
</script>
</body>
</html>
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_dispatch_toggles_pause_and_updates_config() {
        let ctrl = Control::new("box-1".into(), "https://mana.deimos.studio".into(), "strict-1".into(), 2, true);
        let (st, _, _) = dispatch("GET", "/", &[], &ctrl);
        assert_eq!(st, "200 OK");

        let (st, _, _) = dispatch("POST", "/api/pause", &[], &ctrl);
        assert_eq!(st, "200 OK");
        assert!(ctrl.paused.load(Ordering::Relaxed));

        let (st, _, _) = dispatch("POST", "/api/config", br#"{"jobs":1,"idle_only":false}"#, &ctrl);
        assert_eq!(st, "200 OK");
        assert_eq!(ctrl.jobs.load(Ordering::Relaxed), 1);
        assert!(!ctrl.idle_only.load(Ordering::Relaxed));

        let (st, _, _) = dispatch("POST", "/api/resume", &[], &ctrl);
        assert_eq!(st, "200 OK");
        assert!(!ctrl.paused.load(Ordering::Relaxed));
    }
}
