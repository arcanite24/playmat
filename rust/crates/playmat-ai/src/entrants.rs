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
///   `playout_cap`, `policy`, `threads`, `hide`, `samples`, `playout`, `eval`, `plans`, `mulligan_sim`, `blunder`, `net`, `temp`, `eta`, `rollout`, `consider`, `cpuct`, `iters`).
///   For A/B runs: `cfg:hide=1+samples=4` against `cfg:hide=1`.
pub fn entrant(name: &str) -> Result<Entrant, String> {
    // the named entrants are the first search's settings, pinned so their recorded results keep meaning the same
    // games; `host@sN` follows what the host plays now
    let full =
        Config { threads: 4, policy: PolicyKind::Playouts, extra_turns: 0, playout: 1, eval: 1, ..Config::default() };
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
    let mut hide_explicit = false;
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
            "hide" => {
                c.hide = b()?;
                hide_explicit = true;
            }
            "samples" => c.samples = n()?.max(1) as usize,
            "playout" => c.playout = n()? as u8,
            "eval" => c.eval = n()? as u8,
            "plans" => c.plans = b()?,
            "mulligan_sim" => c.mulligan_sim = b()?,
            "blunder" => c.blunder = n()? as u32,
            "net" => c.net = Some(v.to_string()),
            "temp" => c.temp = n()? as u32,
            "eta" => c.eta = n()? as u32,
            "rollout" => c.rollout = (v != "-").then(|| v.to_string()),
            "consider" => c.consider = n()? as usize,
            "cpuct" => c.cpuct = n()? as u32,
            "iters" => c.iters = n()? as u32,
            "playbook" => c.playbook = (v != "-").then(|| v.to_string()),
            "leaf_blend" => c.leaf_blend = n()?.min(1000) as u32,
            "gumbel_root" => c.gumbel_root = b()?,
            "temp_turn" => c.temp_turn = n()? as u32,
            "time_ms" => c.time_ms = n()? as u32,
            "c_scale" => c.c_scale = n()? as u32,
            "par" => c.par = n()?.max(1) as u32,
            "multi_tree" => c.multi_tree = b()?,
            "quiesce_net" => c.quiesce_net = b()?,
            "mulligan_net" => c.mulligan_net = b()?,
            "belief_ms" => c.belief_ms = b()?,
            "cfr_root" => c.cfr_root = n()? as u32,
            "vloss" => c.vloss = n()?.max(1) as usize,
            "rm_root" => c.rm_root = b()?,
            _ => return Err(format!("unknown config key {k}")),
        }
    }
    if !hide_explicit && matches!(c.policy, PolicyKind::NetSearch | PolicyKind::Gumbel | PolicyKind::Ismcts) {
        c.hide = true;
    }
    Ok(c)
}

/// The settings `label` names only where they differ from their defaults, in its order
pub const LATER: [&str; 14] = [
    "playbook",
    "leaf_blend",
    "gumbel_root",
    "temp_turn",
    "time_ms",
    "c_scale",
    "par",
    "multi_tree",
    "quiesce_net",
    "mulligan_net",
    "belief_ms",
    "cfr_root",
    "vloss",
    "rm_root",
];

/// A config as `cfg:` settings, so two builds' configs compare as text: every field up to `iters`, then the later ones
/// only where they differ from their defaults, so a config that doesn't use them keeps the label it was pinned with
/// (the ladder's bots, `ladder-calibrate`)
pub fn label(c: &Config) -> String {
    let mut s = format!(
        "budget={}+width={}+extra_turns={}+playout_cap={}+policy={}+hide={}+samples={}+playout={}+eval={}+plans={}+mulligan_sim={}+blunder={}+net={}+temp={}+eta={}+rollout={}+consider={}+cpuct={}+iters={}",
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
        c.blunder,
        c.net.as_deref().unwrap_or("builtin"),
        c.temp,
        c.eta,
        c.rollout.as_deref().unwrap_or("-"),
        c.consider,
        c.cpuct,
        c.iters,
    );
    if let Some(pb) = c.playbook.as_deref() {
        s.push_str(&format!("+playbook={pb}"));
    }
    let later: [(u64, u64); 13] = [
        (c.leaf_blend as u64, 0),
        (c.gumbel_root as u64, 0),
        (c.temp_turn as u64, 0),
        (c.time_ms as u64, 0),
        (c.c_scale as u64, 250),
        (c.par as u64, 1),
        (c.multi_tree as u64, 0),
        (c.quiesce_net as u64, 0),
        (c.mulligan_net as u64, 0),
        (c.belief_ms as u64, 0),
        (c.cfr_root as u64, 0),
        (c.vloss as u64, 1),
        (c.rm_root as u64, 0),
    ];
    for (k, (v, default)) in LATER[1..].iter().zip(later) {
        if v != default {
            s.push_str(&format!("+{k}={v}"));
        }
    }
    s
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

/// A config pinned by its `label` (#435): a ladder bot plays the search settings it was rated with, whatever later
/// builds make of its skill. `"quick"` is the quick answers alone. A label pins settings, not code: a change to the
/// evaluation, the built-in net or the engine still moves a pinned bot.
pub fn pinned(label: &str) -> Result<Option<Config>, String> {
    if label == "quick" {
        return Ok(None);
    }
    let mut c = with(Config::default(), label)?;
    if c.net.as_deref() == Some("builtin") {
        c.net = None;
    }
    Ok(Some(c))
}

/// What a seat plays: its pinned config when it has one, else its skill as this build plays it
pub fn seat(skill_now: Option<u32>, pin: Option<&str>) -> Option<Config> {
    match pin.map(pinned) {
        Some(Ok(config)) => config,
        // a pin this build can't read plays the skill, rather than refusing the game
        _ => skill(skill_now),
    }
}

#[cfg(test)]
mod pin_tests {
    use super::*;

    /// every skill survives its label exactly, so pinning today's ladder bots changes none of their games
    #[test]
    fn a_skill_round_trips_through_its_label() {
        for n in 0..=10 {
            let now = skill(Some(n));
            let text = now.as_ref().map(label).unwrap_or_else(|| "quick".to_string());
            let back = pinned(&text).unwrap();
            assert_eq!(format!("{now:?}"), format!("{back:?}"), "skill {n}");
        }
    }

    /// the label names every setting but the threads (the later ones where they're set), so a new `Config` field
    /// can't be left out of a pin
    #[test]
    fn the_label_names_every_setting() {
        let debug = format!("{:?}", Config::default());
        let text = label(&Config::default());
        let inner = &debug[debug.find('{').unwrap() + 1..debug.rfind('}').unwrap()];
        for field in inner.split(", ").filter_map(|kv| kv.split_once(": ").map(|(k, _)| k.trim())) {
            if field == "threads" || !field.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
                continue;
            }
            assert!(text.contains(&format!("{field}=")) || LATER.contains(&field), "{field} is missing from the label");
        }
    }

    /// a later setting that's set shows, and round-trips
    #[test]
    fn a_later_setting_shows_when_set() {
        let c = with(Config::default(), "vloss=8+multi_tree=1").unwrap();
        let text = label(&c);
        assert!(text.ends_with("+multi_tree=1+vloss=8"), "{text}");
        assert_eq!(format!("{:?}", pinned(&text).unwrap()), format!("{:?}", Some(c)));
    }

    #[test]
    fn a_seat_plays_its_pin_or_its_skill() {
        let light = label(&skill(Some(3)).unwrap());
        assert_eq!(format!("{:?}", seat(Some(7), Some(&light))), format!("{:?}", skill(Some(3))));
        assert!(seat(Some(7), Some("quick")).is_none());
        assert_eq!(format!("{:?}", seat(Some(5), Some("nonsense"))), format!("{:?}", skill(Some(5))));
        assert_eq!(format!("{:?}", seat(Some(5), None)), format!("{:?}", skill(Some(5))));
    }
}
