//! Named AI configurations for the benches (Archbench, the tactics suite). Names are versioned with what they play
//! like: a change to an entrant's search that changes its games gets a new name (or a new `@v`), so recorded results
//! keep meaning what they said.

use crate::decision::PolicyKind;
use crate::search::Config;

/// An entrant: a name and the seat's config (`None`: the quick answers alone)
#[derive(Clone, Debug)]
pub struct Entrant {
    pub name: String,
    pub config: Option<Config>,
}

/// The configs behind the names.
///
/// - `rust-playouts@s4`, `rust-playouts@full`, `rust-mcts@s4`, `rust-mcts@full`, `rust-quick`, `random`: the first
///   AI's (constructed@1).
/// - `host@sN`: what the Rust host plays at skill N (`skill`), whatever this build makes of it.
/// - `try:<policy>:<budget>:<width>:<extra turns>:<threads>`: tuning, on skill 4's base.
/// - `cfg:<key>=<value>+…`: any `Config` field on skill 4's first base (`budget`, `width`, `extra_turns`,
///   `playout_cap`, `policy`, `threads`, `hide`, `samples`, `playout`, `eval`, `plans`, `mulligan_sim`, `blunder`).
///   For A/B runs: `cfg:hide=1+samples=4` against `cfg:hide=1`.
pub fn entrant(name: &str) -> Result<Entrant, String> {
    // the named entrants are the first search's settings, pinned so their recorded results keep meaning the same
    // games; `host@sN` follows what the host plays now
    let full = Config {
        threads: 4,
        policy: PolicyKind::Playouts,
        extra_turns: 0,
        playout: 1,
        eval: 1,
        ..Config::default()
    };
    // the Rust host's first skill 4: half the budget, 8 candidates
    let s4 = Config { budget: full.budget / 2, width: 8, ..full.clone() };
    let config = match name {
        "rust-playouts@s4" => Some(s4),
        "rust-playouts@full" => Some(full),
        "rust-mcts@s4" => Some(Config { policy: PolicyKind::Mcts, ..s4 }),
        "rust-mcts@full" => Some(Config { policy: PolicyKind::Mcts, ..full }),
        "rust-quick" => None,
        "random" => Some(Config { policy: PolicyKind::Random, ..full }),
        _ if name.starts_with("host@s") => {
            let n: u32 = name["host@s".len()..].parse().map_err(|_| format!("bad skill in {name}"))?;
            skill(Some(n)).map(|c| Config { threads: 4, ..c })
        }
        _ if name.starts_with("try:") => {
            let p: Vec<&str> = name.split(':').collect();
            let num = |i: usize, d: u64| p.get(i).and_then(|v| v.parse().ok()).unwrap_or(d);
            Some(Config {
                policy: p.get(1).and_then(|v| v.parse().ok()).unwrap_or_default(),
                budget: num(2, s4.budget),
                width: num(3, s4.width as u64) as usize,
                extra_turns: num(4, 0) as u32,
                threads: num(5, 4) as usize,
                ..s4
            })
        }
        _ if name.starts_with("cfg:") => Some(with(s4, &name[4..])?),
        _ => return Err(format!("unknown entrant {name}")),
    };
    Ok(Entrant { name: name.to_string(), config })
}

/// `config` with `key=value` settings joined by `+`
pub fn with(mut c: Config, settings: &str) -> Result<Config, String> {
    for kv in settings.split('+').filter(|s| !s.is_empty()) {
        let (k, v) = kv.split_once('=').ok_or_else(|| format!("{kv}: key=value"))?;
        let n = || v.parse::<u64>().map_err(|_| format!("{kv}: not a number"));
        let b = || Ok::<bool, String>(matches!(v, "1" | "true" | "yes" | "on"));
        match k {
            "budget" => c.budget = n()?,
            "width" => c.width = n()? as usize,
            "extra_turns" => c.extra_turns = n()? as u32,
            "playout_cap" => c.playout_cap = n()?,
            "policy" => c.policy = v.parse()?,
            "threads" => c.threads = n()? as usize,
            "hide" => c.hide = b()?,
            "samples" => c.samples = n()?.max(1) as usize,
            "playout" => c.playout = n()? as u8,
            "eval" => c.eval = n()? as u8,
            "plans" => c.plans = b()?,
            "mulligan_sim" => c.mulligan_sim = b()?,
            "blunder" => c.blunder = n()? as u32,
            _ => return Err(format!("unknown config key {k}")),
        }
    }
    Ok(c)
}

/// A config as `cfg:` settings (every field, so two builds' configs compare as text)
pub fn label(c: &Config) -> String {
    format!(
        "budget={}+width={}+extra_turns={}+playout_cap={}+policy={}+hide={}+samples={}+playout={}+eval={}+plans={}+mulligan_sim={}+blunder={}",
        c.budget,
        c.width,
        c.extra_turns,
        c.playout_cap,
        c.policy.name(),
        c.hide as u8,
        c.samples,
        c.playout,
        c.eval,
        c.plans as u8,
        c.mulligan_sim as u8,
        c.blunder
    )
}

/// The AI a table's skill plays (`TableRules.ai_skill`, 1–10 as Java's; `None`: the default, 6). Skills 0–1 play the
/// quick answers alone.
pub fn skill(skill: Option<u32>) -> Option<Config> {
    let full = Config::default();
    match skill.unwrap_or(6) {
        0..=1 => None,
        // a fifth of the budget, this turn only, Java's score, and a random answer one search in ten
        2..=3 => Some(Config { budget: full.budget / 5, width: 5, extra_turns: 0, eval: 1, blunder: 100, ..full }),
        // half the budget, this turn only, one random answer in fifty
        4..=5 => Some(Config { budget: full.budget / 2, width: 8, extra_turns: 0, blunder: 20, ..full }),
        _ => Some(full),
    }
}
