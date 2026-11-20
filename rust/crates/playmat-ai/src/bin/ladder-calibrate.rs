//! Calibrates the ladder's bots (#430): plays them against each other on the Rust engine and fits Bradley–Terry
//! ratings, one scale per format, anchored so the format's starter at skill 1 is 1000. A bot is a school deck at the
//! AI skill the Rust host plays (`entrants::skill`), listed in `Mage.Server/src/main/resources/ladder/bots.json`
//! (made by `Mage.Server/ladder-src/roster.py`).
//!
//! cargo run --release -p playmat-ai --bin ladder-calibrate -- [--format standard|pioneer|modern|all] [--rounds 2]
//!     [--near 6] [--far 2] [--deals 2] [--jobs 3] [--fit-only] [--fixed] [--log FILE]
//!
//! A bot plays its pinned search settings (`ai` in bots.json, `entrants::pinned`) when it has them, else its skill as
//! this build plays it. `--fixed` (#435, for a new AI generation) keeps every calibrated bot's rating as it is and
//! fits only the uncalibrated ones against them, playing only pairs with a new bot in them.
//!
//! Each round pairs every bot with its `--near` closest bots by the current estimate and `--far` others spread over
//! the scale (so the scale stays connected end to end), and plays each pair until it has `--deals` deals, each twice
//! with the seats swapped. Games go to a log (`Mage.Server/ladder-src/calibration/games.jsonl` by default) as they
//! finish, so a run can stop and go on later; a game counts only while both bots' AI settings are the ones this
//! build plays (`entrants::label`), so a change to the search makes the old games stale. After each round the fit
//! is written back into bots.json: `rating`, `rd` (the fit's standard error), games and results.

use playmat_ai::duel::{new_duel, Deck};
use playmat_ai::entrants::{label, pinned, skill};
use playmat_ai::eval;
use playmat_ai::mulligan;
use playmat_ai::search::{Config, Table};
use playmat_engine::card::CardDb;
use playmat_engine::rules;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::Write;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

/// the game is a draw when this turn would begin (Archbench's cap)
const TURN_CAP: u32 = 30;
/// the narrowest standard error a bot's rating is given, so live results can still move it
const MIN_RD: f64 = 30.0;

struct Bot {
    id: String,
    format: String,
    skill: u32,
    prior: f64,
    deck: Deck,
    config: Option<Config>,
    /// what the bot's AI plays like in this build
    ai: String,
    /// `--fixed`: its calibrated rating, kept as it is
    fixed: Option<f64>,
}

/// A game in the log: the bots by seat (seat 0 starts), the deal's seed, and the winning seat (`None`: a draw)
#[derive(Clone)]
struct Game {
    seats: [String; 2],
    ai: [String; 2],
    seed: i64,
    winner: Option<usize>,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let opt = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned();
    let num = |name: &str, d: usize| opt(name).and_then(|v| v.parse().ok()).unwrap_or(d);
    let repo = env!("CARGO_MANIFEST_DIR").to_string() + "/../../..";
    let roster_path = format!("{repo}/Mage.Server/src/main/resources/ladder/bots.json");
    let decks_dir = format!("{repo}/Mage.Server/src/main/resources/career/campaigns/decks");
    let log_path = opt("--log").unwrap_or_else(|| format!("{repo}/Mage.Server/ladder-src/calibration/games.jsonl"));
    let only = opt("--format").unwrap_or_else(|| "all".into());
    let rounds = num("--rounds", 2);
    let near = num("--near", 6);
    let far = num("--far", 2);
    let deals = num("--deals", 2);
    let jobs = num("--jobs", 3).max(1);
    let fit_only = args.iter().any(|a| a == "--fit-only");
    let fixed = args.iter().any(|a| a == "--fixed");

    let mut roster: Value =
        serde_json::from_str(&std::fs::read_to_string(&roster_path).expect("bots.json")).expect("bots.json");
    let db = playmat_engine::cards();
    let mut bots: Vec<Bot> = Vec::new();
    for b in roster["bots"].as_array().expect("bots") {
        let format = b["format"].as_str().unwrap().to_string();
        if only != "all" && only != format {
            continue;
        }
        let file = b["deck"].as_str().unwrap();
        let text = std::fs::read_to_string(format!("{decks_dir}/{file}")).unwrap_or_else(|_| panic!("deck {file}"));
        let deck = Deck::parse(file, &text);
        let id = b["id"].as_str().unwrap().to_string();
        let missing = deck.missing(&db);
        if !missing.is_empty() {
            println!("LADDER {id}: left out, the engine lacks {}", missing.join(", "));
            continue;
        }
        let s = b["skill"].as_u64().unwrap() as u32;
        // its pin, or what this build plays at its skill
        let config = match b["ai"].as_str() {
            Some(pin) => pinned(pin).unwrap_or_else(|e| panic!("{id}'s ai: {e}")),
            None => skill(Some(s)),
        };
        // one thread a seat: the jobs are the parallelism, and a seat's games don't depend on its threads
        let config = config.map(|c| Config { threads: 1, ..c });
        let ai = config.as_ref().map(label).unwrap_or_else(|| "quick".into());
        let calibrated = b["games"].as_u64().unwrap_or(0) > 0;
        let fixed = if fixed && calibrated { b["rating"].as_f64() } else { None };
        bots.push(Bot { id, format, skill: s, prior: b["prior"].as_f64().unwrap_or(1000.0), deck, config, ai, fixed });
    }
    let anchors: HashMap<String, String> = roster["anchors"]
        .as_object()
        .map(|m| m.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string())).collect())
        .unwrap_or_default();
    let formats: Vec<String> = {
        let mut f: Vec<String> = bots.iter().map(|b| b.format.clone()).collect();
        f.dedup();
        f
    };

    let mut log = read_log(&log_path);
    let start = Instant::now();
    for round in 0..if fit_only { 0 } else { rounds } {
        for format in &formats {
            let members: Vec<usize> = (0..bots.len()).filter(|i| bots[*i].format == *format).collect();
            let fit = fit_format(&bots, &members, &log, anchors.get(format));
            let pairs: Vec<(usize, usize)> = schedule(&bots, &members, &fit, near, far)
                .into_iter()
                .filter(|(x, y)| bots[*x].fixed.is_none() || bots[*y].fixed.is_none())
                .collect();
            // every game still to play: (pair, deal, seats swapped)
            let mut todo = Vec::new();
            for &(x, y) in &pairs {
                let have: HashSet<(i64, bool)> = log
                    .iter()
                    .filter(|g| counts(g, &bots[x], &bots[y]) || counts(g, &bots[y], &bots[x]))
                    .map(|g| (g.seed, g.seats[0] == bots[x].id))
                    .collect();
                for d in 0..deals {
                    let seed = deal_seed(&bots[x].id, &bots[y].id, d);
                    for swapped in [false, true] {
                        if !have.contains(&(seed, !swapped)) {
                            todo.push((x, y, seed, swapped));
                        }
                    }
                }
            }
            println!(
                "LADDER round {} {format}: {} bots, {} pairs, {} games to play",
                round + 1,
                members.len(),
                pairs.len(),
                todo.len()
            );
            let played = play_all(&db, &bots, &todo, jobs, &log_path);
            log.extend(played);
            println!("LADDER round {} {format} done, {:.0} s so far", round + 1, start.elapsed().as_secs_f64());
        }
    }

    // the fit, written back
    let mut fitted: HashMap<String, Fit> = HashMap::new();
    for format in &formats {
        let members: Vec<usize> = (0..bots.len()).filter(|i| bots[*i].format == *format).collect();
        let fit = fit_format(&bots, &members, &log, anchors.get(format));
        let mut order = members.clone();
        order.sort_by(|a, b| fit[b].rating.total_cmp(&fit[a].rating));
        for &i in &order {
            let f = &fit[&i];
            println!(
                "LADDER {:<44} skill {}  rating {:>5.0} ± {:>3.0}  {:>3}-{:>2}-{:<3}",
                bots[i].id, bots[i].skill, f.rating, f.rd, f.won, f.drew, f.lost
            );
            fitted.insert(bots[i].id.clone(), f.clone());
        }
    }
    let mut total = 0;
    let kept: HashSet<&str> = bots.iter().filter(|b| b.fixed.is_some()).map(|b| b.id.as_str()).collect();
    for b in roster["bots"].as_array_mut().unwrap() {
        let Some(f) = fitted.get(b["id"].as_str().unwrap()) else { continue };
        if f.games() == 0 || kept.contains(b["id"].as_str().unwrap()) {
            continue;
        }
        total += f.games();
        b["rating"] = json!(f.rating.round() as i64);
        b["rd"] = json!(f.rd.round() as i64);
        b["games"] = json!(f.games());
        b["won"] = json!(f.won);
        b["drew"] = json!(f.drew);
        b["lost"] = json!(f.lost);
    }
    let commit = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    // what this build plays at each skill; a pinned bot's own settings are its `ai`
    let mut skills = BTreeMap::new();
    for b in &bots {
        let now = skill(Some(b.skill)).map(|c| label(&c)).unwrap_or_else(|| "quick".into());
        skills.insert(b.skill.to_string(), now);
    }
    let mut runs: Vec<Value> = roster["calibration"]["runs"].as_array().cloned().unwrap_or_default();
    if !fit_only {
        // one run per set of formats: a run of one format replaces only that format's record, the others keep
        // theirs; a --fixed run adds to them, since the bots it kept keep their measurement
        if !fixed {
            runs.retain(|r| {
                let theirs: Vec<String> = r["formats"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|f| f.as_str().map(String::from))
                    .collect();
                !theirs.iter().any(|f| formats.contains(f))
            });
        }
        let new_games: u32 =
            bots.iter().filter(|b| b.fixed.is_none()).filter_map(|b| fitted.get(&b.id)).map(|f| f.games()).sum();
        runs.push(json!({
            "formats": formats,
            "commit": commit,
            "games": if fixed { new_games } else { total / 2 },
            "settings": format!("{rounds} rounds, --near {near} --far {far} --deals {deals}{}", if fixed { " --fixed" } else { "" }),
        }));
    }
    let games: u64 = runs.iter().filter_map(|r| r["games"].as_u64()).sum();
    // a fit alone measured nothing new: the record keeps the build that played the games
    let commit =
        if fit_only { roster["calibration"]["commit"].as_str().unwrap_or(&commit).to_string() } else { commit };
    roster["calibration"] = json!({
        "commit": commit,
        "harness": "rust-engine",
        "turnCap": TURN_CAP,
        "mulligans": true,
        "games": games,
        "skills": skills,
        "runs": runs,
    });
    let mut text = serde_json::to_string_pretty(&roster).unwrap();
    text.push('\n');
    std::fs::write(&roster_path, text).expect("write bots.json");
    println!("LADDER wrote {roster_path} ({} games)", total / 2);
}

/// Whether a logged game is bot `a` (seat 0) against bot `b` (seat 1) as they play in this build
fn counts(g: &Game, a: &Bot, b: &Bot) -> bool {
    g.seats[0] == a.id && g.seats[1] == b.id && g.ai[0] == a.ai && g.ai[1] == b.ai
}

/// The pairs to play: each bot with its `near` closest by estimate, and `far` more spread over the whole scale
fn schedule(
    bots: &[Bot],
    members: &[usize],
    fit: &HashMap<usize, Fit>,
    near: usize,
    far: usize,
) -> Vec<(usize, usize)> {
    let mut order = members.to_vec();
    order.sort_by(|a, b| fit[a].rating.total_cmp(&fit[b].rating).then(bots[*a].id.cmp(&bots[*b].id)));
    let n = order.len();
    let mut pairs: HashSet<(usize, usize)> = HashSet::new();
    let mut add = |x: usize, y: usize| {
        if x != y {
            pairs.insert((x.min(y), x.max(y)));
        }
    };
    for (pos, &x) in order.iter().enumerate() {
        // the closest by distance in rating, from both sides
        let mut by_distance: Vec<usize> = order.iter().copied().filter(|y| *y != x).collect();
        by_distance
            .sort_by(|a, b| (fit[a].rating - fit[&x].rating).abs().total_cmp(&(fit[b].rating - fit[&x].rating).abs()));
        for &y in by_distance.iter().take(near) {
            add(x, y);
        }
        // spread: evenly spaced positions, offset by the bot's own position
        for k in 1..=far {
            add(x, order[(pos + k * n / (far + 1)) % n]);
        }
    }
    let mut out: Vec<(usize, usize)> = pairs.into_iter().collect();
    out.sort();
    out
}

#[derive(Clone, Default)]
struct Fit {
    rating: f64,
    rd: f64,
    won: u32,
    drew: u32,
    lost: u32,
}

impl Fit {
    fn games(&self) -> u32 {
        self.won + self.drew + self.lost
    }
}

/// Bradley–Terry over a format's logged games (a draw is half a win each way, and half a game of prior against the
/// bot's prior keeps an unbeaten bot finite), on the Elo scale with the anchor at 1000. Bots without games keep their
/// prior.
fn fit_format(bots: &[Bot], members: &[usize], log: &[Game], anchor: Option<&String>) -> HashMap<usize, Fit> {
    let index: HashMap<&str, usize> = members.iter().map(|i| (bots[*i].id.as_str(), *i)).collect();
    // (x, y) -> x's score against y and the games between them
    let mut score: HashMap<(usize, usize), (f64, f64)> = HashMap::new();
    let mut out: HashMap<usize, Fit> =
        members.iter().map(|i| (*i, Fit { rating: bots[*i].prior, ..Fit::default() })).collect();
    for g in log {
        let (Some(&a), Some(&b)) = (index.get(g.seats[0].as_str()), index.get(g.seats[1].as_str())) else { continue };
        if !counts(g, &bots[a], &bots[b]) {
            continue;
        }
        let sa = match g.winner {
            Some(0) => 1.0,
            Some(_) => 0.0,
            None => 0.5,
        };
        for (x, y, s) in [(a, b, sa), (b, a, 1.0 - sa)] {
            let e = score.entry((x, y)).or_default();
            e.0 += s;
            e.1 += 1.0;
            let f = out.get_mut(&x).unwrap();
            match s {
                1.0 => f.won += 1,
                0.0 => f.lost += 1,
                _ => f.drew += 1,
            }
        }
    }
    let played: Vec<usize> = members.iter().copied().filter(|i| out[i].games() > 0).collect();
    if played.is_empty() {
        return out;
    }
    // strengths on the natural scale, started at the priors
    let to_p = |elo: f64| 10f64.powf(elo / 400.0);
    let start = |i: usize| bots[i].fixed.unwrap_or(bots[i].prior) - 1000.0;
    let mut p: HashMap<usize, f64> = played.iter().map(|i| (*i, to_p(start(*i)))).collect();
    let prior_p: HashMap<usize, f64> = p.clone();
    // --fixed: the calibrated bots are the scale, so they don't move and the anchor isn't moved to 1000 again
    let any_fixed = played.iter().any(|i| bots[*i].fixed.is_some());
    for _ in 0..3000 {
        let mut q = p.clone();
        for &x in &played {
            if bots[x].fixed.is_some() {
                continue;
            }
            // half a game against a phantom at the bot's own prior
            let mut wins = 0.25;
            let mut denom = 0.5 / (p[&x] + prior_p[&x]);
            for &y in &played {
                if let Some(&(s, n)) = score.get(&(x, y)) {
                    wins += s;
                    denom += n / (p[&x] + p[&y]);
                }
            }
            q.insert(x, wins / denom);
        }
        p = q;
    }
    let elo: HashMap<usize, f64> = p.iter().map(|(i, v)| (*i, 400.0 * v.log10())).collect();
    let zero = if any_fixed {
        0.0
    } else {
        anchor
            .and_then(|a| index.get(a.as_str()))
            .and_then(|a| elo.get(a))
            .copied()
            .unwrap_or_else(|| elo.values().sum::<f64>() / elo.len() as f64)
    };
    let k = std::f64::consts::LN_10 / 400.0;
    for &x in &played {
        // the standard error from the fit's information: Σ n·p(1-p), on the Elo scale
        let mut info = 0.0;
        for &y in &played {
            if let Some(&(_, n)) = score.get(&(x, y)) {
                let e = p[&x] / (p[&x] + p[&y]);
                info += n * e * (1.0 - e);
            }
        }
        let f = out.get_mut(&x).unwrap();
        f.rating = elo[&x] - zero + 1000.0;
        f.rd = if info > 0.0 { (1.0 / (info * k * k)).sqrt().max(MIN_RD) } else { 350.0 };
    }
    out
}

/// A pair's deal seed: the same pair and deal always deal the same cards
fn deal_seed(a: &str, b: &str, deal: usize) -> i64 {
    let (lo, hi) = if a < b { (a, b) } else { (b, a) };
    (fnv(&format!("{lo}|{hi}|{deal}")) >> 1) as i64
}

/// Plays the games on `jobs` threads, appending each to the log as it finishes
fn play_all(db: &CardDb, bots: &[Bot], todo: &[(usize, usize, i64, bool)], jobs: usize, log_path: &str) -> Vec<Game> {
    if let Some(dir) = std::path::Path::new(log_path).parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let file = Mutex::new(std::fs::OpenOptions::new().create(true).append(true).open(log_path).expect("log"));
    let out = Mutex::new(Vec::new());
    let next = AtomicUsize::new(0);
    let done = AtomicUsize::new(0);
    let start = Instant::now();
    std::thread::scope(|s| {
        for _ in 0..jobs {
            s.spawn(|| loop {
                let g = next.fetch_add(1, Ordering::Relaxed);
                let Some(&(x, y, seed, swapped)) = todo.get(g) else { break };
                let (a, b) = if swapped { (y, x) } else { (x, y) };
                let Some(winner) = play(db, [&bots[a], &bots[b]], seed) else {
                    println!("LADDER {} vs {} seed {seed}: failed", bots[a].id, bots[b].id);
                    continue;
                };
                let game = Game {
                    seats: [bots[a].id.clone(), bots[b].id.clone()],
                    ai: [bots[a].ai.clone(), bots[b].ai.clone()],
                    seed,
                    winner,
                };
                let line = json!({"a": game.seats[0], "b": game.seats[1], "aiA": game.ai[0], "aiB": game.ai[1],
                    "seed": seed, "winner": winner});
                writeln!(file.lock().unwrap(), "{line}").expect("log write");
                out.lock().unwrap().push(game);
                let n = done.fetch_add(1, Ordering::Relaxed) + 1;
                if n.is_multiple_of(50) || n == todo.len() {
                    let secs = start.elapsed().as_secs_f64();
                    println!(
                        "LADDER {n}/{} games, {:.1} s a game, about {:.0} min left",
                        todo.len(),
                        secs / n as f64,
                        secs / n as f64 * (todo.len() - n) as f64 / 60.0
                    );
                }
            });
        }
    });
    out.into_inner().unwrap()
}

/// One game, seat 0 starting: the winning seat (`None` for a draw), or `None` overall when it failed
fn play(db: &CardDb, bots: [&Bot; 2], seed: i64) -> Option<Option<usize>> {
    let mut game = new_duel(db, [&bots[0].deck, &bots[1].deck], seed, 0).ok()?;
    let configs = [bots[0].config.clone(), bots[1].config.clone()];
    let run = catch_unwind(AssertUnwindSafe(|| {
        mulligan::london(
            &mut game,
            0,
            0,
            &mut |g, seat, taken| mulligan::wants(g, seat, taken, configs[seat].as_ref()),
            &mut |g, seat| mulligan::bottom(g, seat, configs[seat].as_ref()),
        );
        let mut table = Table::new(configs.to_vec());
        rules::play_from_priority(&mut game, &mut table, TURN_CAP);
        table
    }));
    let table = run.ok()?;
    if table.stopped_early() {
        return None;
    }
    Some(if game.player(0).lost || eval::evaluate(&game, 1) == eval::WIN {
        Some(1)
    } else if game.player(1).lost || eval::evaluate(&game, 0) == eval::WIN {
        Some(0)
    } else {
        None
    })
}

fn read_log(path: &str) -> Vec<Game> {
    let Ok(text) = std::fs::read_to_string(path) else { return Vec::new() };
    text.lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .map(|v| Game {
            seats: [v["a"].as_str().unwrap_or("").into(), v["b"].as_str().unwrap_or("").into()],
            ai: [v["aiA"].as_str().unwrap_or("").into(), v["aiB"].as_str().unwrap_or("").into()],
            seed: v["seed"].as_i64().unwrap_or(0),
            winner: v["winner"].as_u64().map(|w| w as usize),
        })
        .collect()
}

/// FNV-1a, 64 bits
fn fnv(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ b as u64).wrapping_mul(0x0100_0000_01b3))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bot(id: &str, prior: f64, fixed: Option<f64>) -> Bot {
        Bot {
            id: id.into(),
            format: "standard".into(),
            skill: 7,
            prior,
            deck: Deck::parse(id, ""),
            config: None,
            ai: "quick".into(),
            fixed,
        }
    }

    fn games(a: &str, b: &str, a_wins: usize, b_wins: usize) -> Vec<Game> {
        let one = |winner| Game { seats: [a.into(), b.into()], ai: ["quick".into(), "quick".into()], seed: 0, winner };
        (0..a_wins).map(|_| one(Some(0))).chain((0..b_wins).map(|_| one(Some(1)))).collect()
    }

    /// --fixed (#435): the calibrated bots keep their ratings and the new one lands on their scale
    #[test]
    fn a_new_generation_is_rated_against_the_old_as_they_stand() {
        let bots =
            vec![bot("old-a", 1000.0, Some(1000.0)), bot("old-b", 1400.0, Some(1400.0)), bot("new", 1100.0, None)];
        let mut log = games("new", "old-a", 30, 10);
        log.extend(games("new", "old-b", 10, 30));
        // the old bots' games against each other disagree with their ratings, and must not move them
        log.extend(games("old-a", "old-b", 40, 0));
        let fit = fit_format(&bots, &[0, 1, 2], &log, Some(&"old-a".to_string()));
        assert_eq!(fit[&0].rating, 1000.0);
        assert_eq!(fit[&1].rating, 1400.0);
        // 3-1 against 1000 and 1-3 against 1400: halfway
        assert!((fit[&2].rating - 1200.0).abs() < 15.0, "{}", fit[&2].rating);
    }

    /**
     * The newest generation plays what this build plays (#435). When this fails, the Rust AI changed: the ladder's
     * bots keep playing their pins, so add a generation in roster.py with the build's labels (printed here), run it,
     * and `ladder-calibrate --fixed`. A change that is not stronger can instead stay off the ladder for now.
     */
    #[test]
    fn the_newest_generation_plays_this_build() {
        let path = env!("CARGO_MANIFEST_DIR").to_string() + "/../../../Mage.Server/src/main/resources/ladder/bots.json";
        let roster: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let bots = roster["bots"].as_array().unwrap();
        let newest = bots.iter().filter_map(|b| b["generation"].as_u64()).max().expect("bots have generations");
        let mut checked = 0;
        for b in bots.iter().filter(|b| b["generation"].as_u64() == Some(newest)) {
            let n = b["skill"].as_u64().unwrap() as u32;
            let now = skill(Some(n)).map(|c| label(&c)).unwrap_or_else(|| "quick".into());
            assert_eq!(
                b["ai"].as_str().unwrap(),
                now,
                "skill {n} plays differently in this build than generation {newest}: {now}"
            );
            checked += 1;
        }
        assert!(checked > 0);
    }
}
