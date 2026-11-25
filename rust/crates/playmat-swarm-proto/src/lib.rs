//! Mana@Home's protocol (docs/swarm/Design.md, "Projects and work units"): what the coordinator leases to a worker
//! (`WorkUnit`), what the worker sends back (`ResultManifest`), which learner configs a unit may carry
//! (`check_config`), and which machines can reproduce each other's games (`determinism_class`).
//!
//! A unit is data, never code: its configs are checked key by key against an allowlist, and its networks are named by
//! sha256 (`net=@<sha256>`), which the worker resolves to files it fetched and checked itself.

pub mod api;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Bumped whenever a unit or manifest changes shape; a worker refuses units of another version
pub const PROTOCOL: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UnitKind {
    /// self-play games recorded for training
    Selfplay,
    /// self-play with a GPU inference backend (not bit-exact: verified statistically)
    SelfplayGpu,
    /// games between two entrants for a gate or a sweep; nothing recorded
    Match,
    /// an Archbench rating run
    Bench,
    /// a replay or re-run of another account's unit
    Verify,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NetRole {
    Learner,
    Opponent,
    Rollout,
}

/// A network a unit needs, fetched from `url` and accepted only if its sha256 matches
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetRef {
    pub role: NetRole,
    pub sha256: String,
    pub url: String,
}

/// What a worker must match to take the unit
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pins {
    /// worker builds allowed (git commit of the build); empty = any build of the protocol version
    #[serde(default)]
    pub builds: Vec<String>,
    /// the observation version the networks were trained on (Phase 3: 3)
    pub obs: u32,
    /// the suite folder under `archbench/`, and the sha256 of its suite.json
    pub suite: String,
    pub suite_sha256: String,
    /// the unit needs the strict-math build, which makes every CPU worker one determinism class. The swarm's
    /// default (decided after S0, docs/swarm/S0.md): a unit that doesn't say is strict
    #[serde(default = "strict_by_default")]
    pub strict: bool,
}

fn strict_by_default() -> bool {
    true
}

/// One opponent of the learner, drawn by weight per game; `config` is an Archbench entrant, or `self`
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Opponent {
    pub config: String,
    pub weight: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkUnit {
    pub protocol: u32,
    pub id: String,
    pub project: String,
    pub run: String,
    pub round: u32,
    pub kind: UnitKind,
    pub pins: Pins,
    pub nets: Vec<NetRef>,
    /// the learner's config, `cfg:...` with `net=@<sha256>`
    pub learner: String,
    pub opponents: Vec<Opponent>,
    /// deals `seed .. seed + games` of the suite (seeds always come from the unit, never the worker)
    pub seed: u64,
    pub games: u32,
    /// chance per thousand that a decision is recorded, and the policy that answers the others
    pub keep: u32,
    #[serde(default)]
    pub fast: Option<String>,
    /// lease deadline (unix seconds) and the unit's cost in credit units
    pub deadline: u64,
    pub cost: f64,
}

/// One game of a result
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameSummary {
    pub deal: u64,
    /// the winning seat, `None` for a draw or a game cut off at the turn cap
    pub winner: Option<u8>,
    /// which seat the learner had
    pub learner: u8,
    /// the learner's opponent, by its index in the unit's `opponents` (the league's win rates, by opponent)
    #[serde(default)]
    pub opponent: u32,
    pub turns: u32,
    /// every answer of the game (the answer log's length) and the decisions recorded for training
    pub answers: u32,
    pub recorded: u32,
    /// sha256 of the game's answer log: two runs of a game made the same decisions when these match
    pub log_sha256: String,
    /// sha256 of the game's recorded training steps (`playmat_ai::play::steps_hash`): and recorded the same data
    pub steps_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResultManifest {
    pub protocol: u32,
    pub unit: String,
    pub worker: String,
    /// git commit the worker was built from, and its determinism class
    pub build: String,
    pub class: String,
    /// sha256 of each network used, as the worker computed them
    pub nets: Vec<String>,
    pub games: Vec<GameSummary>,
    /// deals that failed (a panic, or the answer guard): not in `games`
    #[serde(default)]
    pub failed: Vec<u64>,
    /// sha256 of each file the result uploads (the shard's columns, the answer logs)
    pub files: BTreeMap<String, String>,
    pub elapsed_ms: u64,
}

/// Config keys a unit's learner and opponents may set. Anything else is refused, and so are the keys that make games
/// depend on the machine: `time_ms` (a wall-clock cap), `threads` and `par` above 1 (thread timing).
pub const ALLOWED_KEYS: &[&str] = &[
    "budget",
    "width",
    "extra_turns",
    "playout_cap",
    "policy",
    "hide",
    "samples",
    "playout",
    "eval",
    "plans",
    "mulligan_sim",
    "blunder",
    "net",
    "temp",
    "eta",
    "rollout",
    "consider",
    "cpuct",
    "iters",
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
    "threads",
];

/// Checks a learner or opponent config for a unit of `kind`: `self`, a named entrant (`host@s6`), or `cfg:k=v+...`
/// with allowlisted keys, networks named by sha256, and nothing machine-dependent.
pub fn check_config(config: &str, kind: UnitKind) -> Result<(), String> {
    if config == "self" || !config.starts_with("cfg:") {
        return if config.chars().all(|c| c.is_ascii_alphanumeric() || "@-_.".contains(c)) && !config.is_empty() {
            Ok(())
        } else {
            Err(format!("{config:?}: not an entrant name"))
        };
    }
    for part in config["cfg:".len()..].split('+').filter(|p| !p.is_empty()) {
        let (k, v) = part.split_once('=').ok_or_else(|| format!("{part:?}: expected key=value"))?;
        if !ALLOWED_KEYS.contains(&k) {
            return Err(format!("{k}: not an allowed key"));
        }
        let n = || v.parse::<u64>().map_err(|_| format!("{k}={v}: not a number"));
        match k {
            "time_ms" if n()? > 0 => return Err("time_ms: a wall-clock cap makes games depend on the machine".into()),
            "threads" | "par" if n()? > 1 => return Err(format!("{k}={v}: more than one thread per game")),
            "vloss" if kind != UnitKind::SelfplayGpu && n()? > 1 => {
                return Err("vloss: only GPU-batched units use virtual loss".into())
            }
            "net" | "rollout" if v == "builtin" || (k == "rollout" && v == "-") => {}
            "net" | "rollout" => {
                let sha = v.strip_prefix('@').ok_or_else(|| format!("{k}={v}: networks are named @<sha256>"))?;
                if sha.len() != 64 || !sha.chars().all(|c| c.is_ascii_hexdigit()) {
                    return Err(format!("{k}={v}: not a sha256"));
                }
            }
            "policy" => {
                if !v.chars().all(|c| c.is_ascii_lowercase()) {
                    return Err(format!("policy={v}: not a policy name"));
                }
            }
            _ => {
                n()?;
            }
        }
    }
    Ok(())
}

/// Checks a whole unit before a worker takes it
pub fn check_unit(u: &WorkUnit) -> Result<(), String> {
    if u.protocol != PROTOCOL {
        return Err(format!("protocol {} (this worker speaks {PROTOCOL})", u.protocol));
    }
    if u.games == 0 || u.games > 10_000 {
        return Err(format!("games {}: 1 to 10,000", u.games));
    }
    if u.keep > 1000 {
        return Err(format!("keep {}: per thousand", u.keep));
    }
    check_config(&u.learner, u.kind)?;
    for o in &u.opponents {
        check_config(&o.config, u.kind)?;
    }
    if u.opponents.iter().map(|o| o.weight).sum::<u64>() == 0 {
        return Err("opponents: no weight".into());
    }
    // every network a config names must be one of the unit's, so the worker never fetches anything else
    let shas: Vec<&str> = u.nets.iter().map(|n| n.sha256.as_str()).collect();
    for cfg in std::iter::once(&u.learner).chain(u.opponents.iter().map(|o| &o.config)) {
        for part in cfg.split('+') {
            if let Some(sha) = part.strip_prefix("net=@").or_else(|| part.strip_prefix("rollout=@")) {
                if !shas.contains(&sha) {
                    return Err(format!("{sha}: a config names a network the unit doesn't list"));
                }
            }
        }
    }
    if !u.pins.suite.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err(format!("suite {:?}: not a suite name", u.pins.suite));
    }
    Ok(())
}

/// sha256 of some bytes, in hex
pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

/// The machines whose games match bit for bit: the target (its maths library behind `exp`, `ln` and `tanh`) and the
/// kernel `matrixmultiply` 0.3 picks for the network's products at run time, in its own order: AVX-512F, then FMA with
/// AVX2, then AVX, then SSE2 on x86; NEON on ARM. A strict-math build computes the same way everywhere, so its class
/// names no machine.
pub fn determinism_class(strict: bool) -> String {
    if strict {
        return "strict-1".into();
    }
    let arch = std::env::consts::ARCH;
    let os = std::env::consts::OS;
    #[cfg(target_arch = "x86_64")]
    let kernel = if std::arch::is_x86_feature_detected!("avx512f") {
        "avx512f"
    } else if std::arch::is_x86_feature_detected!("fma") && std::arch::is_x86_feature_detected!("avx2") {
        "avx2+fma"
    } else if std::arch::is_x86_feature_detected!("avx") {
        "avx"
    } else {
        "sse2"
    };
    #[cfg(target_arch = "aarch64")]
    let kernel = "neon";
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    let kernel = "generic";
    format!("{arch}-{os}+{kernel}")
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHA: &str = "6c9c9009f3f3ca4b7590f387539206c4ea7c424f309b4fb1b59762f3ce26c2e9";

    fn unit() -> WorkUnit {
        WorkUnit {
            protocol: PROTOCOL,
            id: "u1".into(),
            project: "main".into(),
            run: "phase3".into(),
            round: 19,
            kind: UnitKind::Selfplay,
            pins: Pins {
                builds: vec![],
                obs: 3,
                suite: "constructed-2".into(),
                suite_sha256: "x".into(),
                strict: true,
            },
            nets: vec![NetRef { role: NetRole::Learner, sha256: SHA.into(), url: "https://example.invalid/n".into() }],
            learner: format!("cfg:policy=ismcts+net=@{SHA}+hide=1+iters=384+multi_tree=1+belief_ms=1"),
            opponents: vec![Opponent { config: "self".into(), weight: 1 }],
            seed: 19_000_000,
            games: 46,
            keep: 200,
            fast: Some("ismcts:12".into()),
            deadline: 0,
            cost: 1.0,
        }
    }

    #[test]
    fn a_phase3_unit_passes() {
        check_unit(&unit()).unwrap();
        let json = serde_json::to_string(&unit()).unwrap();
        assert_eq!(serde_json::from_str::<WorkUnit>(&json).unwrap(), unit());
    }

    /// The opponents `rust/rl/pfsp_league.py` plays the Phase 3 learner against, as `with_temp` sends them
    #[test]
    fn the_phase3_league_passes() {
        let tail = "+hide=1+iters=256+samples=8+cpuct=1000+leaf_blend=1000+gumbel_root=1+multi_tree=1+mulligan_net=1+width=16+consider=12+extra_turns=2+temp=750+temp_turn=10";
        for spec in [
            format!("cfg:policy=ismcts+net=@{SHA}{tail}"),
            format!("cfg:policy=ismcts+net=@{SHA}+hide=1+iters=128+cpuct=1000+leaf_blend=1000+multi_tree=1+mulligan_sim=1+extra_turns=0+temp=750+temp_turn=10"),
            "host@s6".to_string(),
        ] {
            check_config(&spec, UnitKind::Selfplay).unwrap_or_else(|e| panic!("{spec}: {e}"));
        }
    }

    #[test]
    fn a_unit_is_strict_unless_it_says_otherwise() {
        let mut v = serde_json::to_value(unit()).unwrap();
        v["pins"].as_object_mut().unwrap().remove("strict");
        assert!(serde_json::from_value::<WorkUnit>(v.clone()).unwrap().pins.strict);
        v["pins"]["strict"] = false.into();
        assert!(!serde_json::from_value::<WorkUnit>(v).unwrap().pins.strict);
    }

    #[test]
    fn machine_dependent_and_unknown_settings_are_refused() {
        let k = UnitKind::Selfplay;
        assert!(check_config("cfg:policy=ismcts+time_ms=500", k).is_err());
        assert!(check_config("cfg:policy=ismcts+time_ms=0", k).is_ok());
        assert!(check_config("cfg:policy=ismcts+threads=4", k).is_err());
        assert!(check_config("cfg:policy=ismcts+par=2", k).is_err());
        assert!(check_config("cfg:policy=ismcts+vloss=8", k).is_err());
        assert!(check_config("cfg:policy=ismcts+vloss=8", UnitKind::SelfplayGpu).is_ok());
        assert!(check_config("cfg:policy=ismcts+shell=rm", k).is_err());
        assert!(check_config("cfg:net=/etc/passwd", k).is_err());
        assert!(check_config("cfg:net=tcp://10.0.0.1:1", k).is_err());
        assert!(check_config("host@s6", k).is_ok());
        assert!(check_config("../x", k).is_err());
    }

    #[test]
    fn a_unit_may_only_name_its_own_networks() {
        let mut u = unit();
        u.learner = u.learner.replace(SHA, &"0".repeat(64));
        assert!(check_unit(&u).is_err());
    }

    #[test]
    fn the_class_names_the_machine() {
        let c = determinism_class(false);
        assert!(c.contains(std::env::consts::ARCH));
        assert_eq!(determinism_class(true), "strict-1");
    }
}
