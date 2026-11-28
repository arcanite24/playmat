//! External AI policy adapter (`playmat-ext-v1`): bridges any outside decider into Playmat's `DecisionPolicy`
//! trait over line-delimited JSON (NDJSON) via TCP (`tcp://host:port`) or a child subprocess (`cmd:<command>`).
//!
//! Supported by Archbench, `tactics`, `ai-game`, and `Table::set_policy`, so Jev, frontier/open-weight LLMs,
//! ONNX models, Forge-style heuristics, MCP bridges, or custom research agents can be benchmarked directly on the
//! exact same locked suites and rules engine as Playmat's own models.
//!
//! Fairness guarantee: `build_request` never serializes hidden information (opponent hand cards, library contents
//! or order, or unseen face-down exile). `external_state_shows_nothing_the_seat_cant_see` tests this directly.

use crate::decision::{Choice, Decision, DecisionPolicy};
use crate::obs;
use playmat_engine::card::{AbilityDef, CardType};
use playmat_engine::decide::{Answer, Query};
use playmat_engine::state::{Game, ObjId, Seat, StackItem, Target, Zone};
use serde_json::{json, Value};
#[cfg(not(target_arch = "wasm32"))]
use std::io::{BufRead, BufReader, BufWriter, Write};
#[cfg(not(target_arch = "wasm32"))]
use std::net::TcpStream;
#[cfg(not(target_arch = "wasm32"))]
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::Duration;

const PROTOCOL_VERSION: &str = "playmat-ext-v1";

const NAMED_KEYWORDS: [(&str, AbilityDef); 14] = [
    ("Flying", AbilityDef::FlyingAbility),
    ("Trample", AbilityDef::TrampleAbility),
    ("Deathtouch", AbilityDef::DeathtouchAbility),
    ("Lifelink", AbilityDef::LifelinkAbility),
    ("First Strike", AbilityDef::FirstStrikeAbility),
    ("Double Strike", AbilityDef::DoubleStrikeAbility),
    ("Haste", AbilityDef::HasteAbility),
    ("Vigilance", AbilityDef::VigilanceAbility),
    ("Reach", AbilityDef::ReachAbility),
    ("Menace", AbilityDef::MenaceAbility),
    ("Defender", AbilityDef::DefenderAbility),
    ("Hexproof", AbilityDef::HexproofAbility),
    ("Indestructible", AbilityDef::IndestructibleAbility),
    ("Flash", AbilityDef::FlashAbility),
];

#[cfg(not(target_arch = "wasm32"))]
enum Transport {
    Tcp { reader: BufReader<TcpStream>, writer: BufWriter<TcpStream> },
    Process { child: Child, reader: BufReader<ChildStdout>, writer: BufWriter<ChildStdin> },
}

#[cfg(target_arch = "wasm32")]
enum Transport {}

impl Transport {
    #[cfg(not(target_arch = "wasm32"))]
    fn connect(spec: &str, timeout: Duration) -> Result<Transport, String> {
        if let Some(rest) = spec.strip_prefix("tcp://") {
            let addr = rest.split('?').next().unwrap_or(rest);
            let stream = TcpStream::connect(addr).map_err(|e| format!("external tcp connect {addr}: {e}"))?;
            let _ = stream.set_nodelay(true);
            let _ = stream.set_read_timeout(Some(timeout));
            let _ = stream.set_write_timeout(Some(timeout));
            let read_stream = stream.try_clone().map_err(|e| format!("external tcp clone: {e}"))?;
            return Ok(Transport::Tcp { reader: BufReader::new(read_stream), writer: BufWriter::new(stream) });
        }
        if let Some(cmd) = spec.strip_prefix("cmd:") {
            let mut command = if cfg!(windows) {
                let mut c = Command::new("cmd");
                c.args(["/C", cmd]);
                c
            } else {
                let mut c = Command::new("sh");
                c.args(["-c", cmd]);
                c
            };
            let mut child = command
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::inherit())
                .spawn()
                .map_err(|e| format!("external cmd spawn {cmd:?}: {e}"))?;
            let stdin = child.stdin.take().ok_or_else(|| "external cmd stdin unavailable".to_string())?;
            let stdout = child.stdout.take().ok_or_else(|| "external cmd stdout unavailable".to_string())?;
            return Ok(Transport::Process { child, reader: BufReader::new(stdout), writer: BufWriter::new(stdin) });
        }
        Err(format!("unsupported external spec {spec:?} (expected tcp://host:port or cmd:<command>)"))
    }

    #[cfg(target_arch = "wasm32")]
    fn connect(_spec: &str, _timeout: Duration) -> Result<Transport, String> {
        Err("external policy transport is unavailable on wasm32".to_string())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn round_trip(&mut self, line: &str) -> Result<String, String> {
        let mut resp = String::new();
        match self {
            Transport::Tcp { reader, writer } => {
                writer.write_all(line.as_bytes()).map_err(|e| format!("tcp write: {e}"))?;
                writer.write_all(b"\n").map_err(|e| format!("tcp newline: {e}"))?;
                writer.flush().map_err(|e| format!("tcp flush: {e}"))?;
                let n = reader.read_line(&mut resp).map_err(|e| format!("tcp read: {e}"))?;
                if n == 0 {
                    return Err("tcp eof".to_string());
                }
            }
            Transport::Process { reader, writer, .. } => {
                writer.write_all(line.as_bytes()).map_err(|e| format!("cmd write: {e}"))?;
                writer.write_all(b"\n").map_err(|e| format!("cmd newline: {e}"))?;
                writer.flush().map_err(|e| format!("cmd flush: {e}"))?;
                let n = reader.read_line(&mut resp).map_err(|e| format!("cmd read: {e}"))?;
                if n == 0 {
                    return Err("cmd eof".to_string());
                }
            }
        }
        Ok(resp)
    }

    #[cfg(target_arch = "wasm32")]
    fn round_trip(&mut self, _line: &str) -> Result<String, String> {
        match *self {}
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Drop for Transport {
    fn drop(&mut self) {
        if let Transport::Process { child, .. } = self {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// External `DecisionPolicy` talking to a TCP server or subprocess adapter over `playmat-ext-v1` NDJSON.
pub struct ExternalPolicy {
    seat: Seat,
    endpoint: Option<String>,
    transport: Option<Transport>,
    include_obs: bool,
    all_queries: bool,
    timeout: Duration,
    last_policy: Vec<f32>,
    last_value: Option<f32>,
    debug: bool,
}

impl ExternalPolicy {
    pub fn new(seat: Seat) -> ExternalPolicy {
        let timeout_ms =
            std::env::var("PLAYMAT_EXT_TIMEOUT_MS").ok().and_then(|v| v.parse::<u64>().ok()).unwrap_or(60_000).max(250);
        let include_obs = std::env::var("PLAYMAT_EXT_OBS").is_ok_and(|v| v == "1" || v == "true");
        let all_queries = std::env::var("PLAYMAT_EXT_ALL_QUERIES").is_ok_and(|v| v == "1" || v == "true");
        let debug = std::env::var("PLAYMAT_EXT_DEBUG").is_ok_and(|v| v == "1" || v == "true");
        ExternalPolicy {
            seat,
            endpoint: None,
            transport: None,
            include_obs,
            all_queries,
            timeout: Duration::from_millis(timeout_ms),
            last_policy: Vec::new(),
            last_value: None,
            debug,
        }
    }

    /// Creates an external policy bound directly to `spec` (`tcp://host:port[?obs=1]` or `cmd:...`).
    pub fn with_endpoint(seat: Seat, spec: impl Into<String>) -> ExternalPolicy {
        let mut p = ExternalPolicy::new(seat);
        p.set_endpoint(spec.into());
        p
    }

    fn set_endpoint(&mut self, spec: String) {
        if let Some(q) = spec.split_once('?').map(|(_, q)| q) {
            for part in q.split('&') {
                if part == "obs=1" || part == "obs=true" {
                    self.include_obs = true;
                } else if part == "all_queries=1" || part == "all_queries=true" {
                    self.all_queries = true;
                }
            }
        }
        self.endpoint = Some(spec);
        self.transport = None;
    }

    fn ensure_connected(&mut self, d: &Decision) -> Result<&mut Transport, String> {
        let desired = d
            .config
            .external
            .as_deref()
            .map(str::to_string)
            .or_else(|| std::env::var("PLAYMAT_EXT_URL").ok())
            .or_else(|| self.endpoint.clone())
            .ok_or_else(|| "external policy has no endpoint (set external=tcp://... or PLAYMAT_EXT_URL)".to_string())?;
        if self.endpoint.as_deref() != Some(desired.as_str()) {
            self.set_endpoint(desired);
        }
        if self.transport.is_none() {
            let spec = self.endpoint.as_deref().unwrap();
            self.transport = Some(Transport::connect(spec, self.timeout)?);
        }
        Ok(self.transport.as_mut().unwrap())
    }
}

impl DecisionPolicy for ExternalPolicy {
    fn name(&self) -> &str {
        "external"
    }

    fn decide(&mut self, d: &Decision) -> Option<Choice> {
        self.last_policy.clear();
        self.last_value = None;
        if !self.all_queries && matches!(d.query, Query::PlayMana { .. } | Query::MultiAmount { .. }) {
            return None;
        }
        let req = build_request(d, self.include_obs);
        let payload = match serde_json::to_string(&req) {
            Ok(s) => s,
            Err(e) => {
                if self.debug {
                    eprintln!("playmat-ext serialize error (seat {}): {e}", self.seat);
                }
                return None;
            }
        };
        let resp_line = match self.ensure_connected(d).and_then(|t| t.round_trip(&payload)) {
            Ok(line) => line,
            Err(e) => {
                if self.debug {
                    eprintln!("playmat-ext transport error (seat {}): {e}", self.seat);
                }
                // drop broken transport so next decision can reconnect
                self.transport = None;
                return None;
            }
        };
        let parsed: Value = match serde_json::from_str(resp_line.trim()) {
            Ok(v) => v,
            Err(e) => {
                if self.debug {
                    eprintln!("playmat-ext invalid JSON response: {e} ({:?})", resp_line.trim());
                }
                return None;
            }
        };
        if let Some(v) = parsed.get("value").and_then(Value::as_f64) {
            if v.is_finite() {
                self.last_value = Some((v as f32).clamp(0.0, 1.0));
            }
        }
        if let Some(arr) = parsed.get("probabilities").and_then(Value::as_array) {
            if arr.len() == d.candidates.len() {
                let probs: Vec<f32> = arr.iter().map(|v| v.as_f64().unwrap_or(0.0) as f32).collect();
                self.last_policy = probs;
            }
        }
        let idx = parse_choice_index(&parsed, d.candidates.len())?;
        let answer = d.candidates.get(idx)?.clone();
        if !d.query.accepts(&answer) {
            return None;
        }
        let nodes = parsed.get("nodes").and_then(Value::as_u64).unwrap_or(1);
        Some(Choice { answer, nodes })
    }

    fn last_policy(&self) -> Option<&[f32]> {
        (!self.last_policy.is_empty()).then_some(self.last_policy.as_slice())
    }

    fn last_value(&self) -> Option<f32> {
        self.last_value
    }
}

/// Parses `"choice"` as either an integer index (`0..k`) or a candidate id string (`"c0"`, `"0"`, etc.).
pub fn parse_choice_index(v: &Value, k: usize) -> Option<usize> {
    let raw = v.get("choice")?;
    let idx = if let Some(n) = raw.as_u64() {
        n as usize
    } else {
        let s = raw.as_str()?;
        let trimmed = s.trim();
        let digits = trimmed.strip_prefix('c').or_else(|| trimmed.strip_prefix('C')).unwrap_or(trimmed);
        digits.parse::<usize>().ok()?
    };
    (idx < k).then_some(idx)
}

/// Serializes a strict imperfect-information decision request for external AIs.
///
/// Opponent hidden hand cards and all library contents/orders are omitted completely.
pub fn build_request(d: &Decision, include_obs: bool) -> Value {
    let g = d.game;
    let seat = d.seat;
    let opp = 1 - seat.min(1);
    let step = g.step.map(|s| s.name()).unwrap_or("NONE");
    let active_player = if g.active == Some(seat) { "you" } else { "opponent" };

    let you_p = g.player(seat);
    let opp_p = g.player(opp);

    let you_hand: Vec<Value> = you_p.hand.iter().map(|&id| card_json(g, id, false)).collect();
    let you_field: Vec<Value> = g
        .battlefield
        .iter()
        .copied()
        .filter(|&id| g.obj(id).controller == seat)
        .map(|id| card_json(g, id, true))
        .collect();
    let opp_field: Vec<Value> = g
        .battlefield
        .iter()
        .copied()
        .filter(|&id| g.obj(id).controller == opp)
        .map(|id| card_json(g, id, true))
        .collect();
    let you_gy: Vec<Value> = you_p.graveyard.iter().map(|&id| card_json(g, id, false)).collect();
    let opp_gy: Vec<Value> = opp_p.graveyard.iter().map(|&id| card_json(g, id, false)).collect();
    let you_exile: Vec<Value> = g
        .exile
        .iter()
        .copied()
        .filter(|&id| {
            let o = g.obj(id);
            o.owner == seat && (!o.face_down || o.foretold_turn.is_some() || o.plotted_turn.is_some())
        })
        .map(|id| card_json(g, id, false))
        .collect();
    let opp_exile: Vec<Value> = g
        .exile
        .iter()
        .copied()
        .filter(|&id| {
            let o = g.obj(id);
            o.owner == opp && !o.face_down
        })
        .map(|id| card_json(g, id, false))
        .collect();

    let stack: Vec<Value> = g.stack.iter().rev().map(|item| stack_item_json(g, seat, item)).collect();

    let attackers: Vec<Value> = g
        .battlefield
        .iter()
        .copied()
        .filter(|&id| g.obj(id).attacking.is_some())
        .map(|id| {
            let o = g.obj(id);
            let (p, t) = g.power_toughness(id);
            json!({
                "id": id.0,
                "name": o.def.name,
                "controller": if o.controller == seat { "you" } else { "opponent" },
                "power": p,
                "toughness": t,
                "blocked": o.blocked,
            })
        })
        .collect();
    let blockers: Vec<Value> = g
        .battlefield
        .iter()
        .copied()
        .filter(|&id| !g.obj(id).blocking.is_empty())
        .map(|id| {
            let o = g.obj(id);
            let (p, t) = g.power_toughness(id);
            json!({
                "id": id.0,
                "name": o.def.name,
                "controller": if o.controller == seat { "you" } else { "opponent" },
                "power": p,
                "toughness": t,
                "blocking_ids": o.blocking.iter().map(|a| a.0).collect::<Vec<_>>(),
            })
        })
        .collect();

    let candidates: Vec<Value> =
        d.candidates.iter().enumerate().map(|(i, a)| candidate_json(g, seat, d.query, a, i, i == 0)).collect();

    let mut req = json!({
        "protocol": PROTOCOL_VERSION,
        "kind": "decide",
        "seat": seat,
        "turn": g.turn,
        "step": step,
        "active_player": active_player,
        "query_kind": query_kind_name(d.query),
        "query_text": query_prompt_text(g, seat, d.query),
        "state": {
            "turn": g.turn,
            "step": step,
            "active_player": active_player,
            "you": {
                "life": you_p.life,
                "lands_played": you_p.lands_played,
                "library_count": you_p.library.len(),
                "mana_pool": mana_json(&you_p.mana_pool),
                "hand": you_hand,
                "battlefield": you_field,
                "graveyard": you_gy,
                "exile": you_exile,
                "counters": you_p.counters,
            },
            "opponent": {
                "life": opp_p.life,
                "lands_played": opp_p.lands_played,
                "hand_count": opp_p.hand.len(),
                "library_count": opp_p.library.len(),
                "mana_pool": mana_json(&opp_p.mana_pool),
                "battlefield": opp_field,
                "graveyard": opp_gy,
                "exile": opp_exile,
                "counters": opp_p.counters,
            },
            "stack": stack,
            "combat": {
                "attackers": attackers,
                "blockers": blockers,
            }
        },
        "candidates": candidates,
    });

    if include_obs {
        let o = obs::observe(g, seat, d.query, d.candidates);
        req["obs"] = json!({
            "glob": o.glob,
            "ents": o.ents,
            "ent_ids": o.ent_ids,
            "ent_rel_idx": o.ent_rel_idx,
            "ent_rel_type": o.ent_rel_type,
            "acts": o.acts,
            "act_ids": o.act_ids,
            "act_ent_idx": o.act_ent_idx,
        });
    }
    req
}

fn mana_json(m: &playmat_engine::card::Mana) -> Value {
    json!({
        "W": m.colored[0],
        "U": m.colored[1],
        "B": m.colored[2],
        "R": m.colored[3],
        "G": m.colored[4],
        "C": m.colorless,
    })
}

fn card_type_name(t: CardType) -> &'static str {
    match t {
        CardType::Artifact => "Artifact",
        CardType::Battle => "Battle",
        CardType::Creature => "Creature",
        CardType::Enchantment => "Enchantment",
        CardType::Instant => "Instant",
        CardType::Kindred => "Kindred",
        CardType::Land => "Land",
        CardType::Planeswalker => "Planeswalker",
        CardType::Sorcery => "Sorcery",
    }
}

fn card_json(g: &Game, id: ObjId, on_field: bool) -> Value {
    let o = g.obj(id);
    let def = &o.def;
    let is_creature =
        if on_field { g.has_type(id, CardType::Creature) } else { def.types.contains(&CardType::Creature) };
    let types: Vec<&'static str> = [
        CardType::Artifact,
        CardType::Battle,
        CardType::Creature,
        CardType::Enchantment,
        CardType::Instant,
        CardType::Land,
        CardType::Planeswalker,
        CardType::Sorcery,
    ]
    .into_iter()
    .filter(|&t| if on_field { g.has_type(id, t) } else { def.types.contains(&t) })
    .map(card_type_name)
    .collect();

    let keywords: Vec<&'static str> = NAMED_KEYWORDS
        .iter()
        .filter(|(_, ab)| if on_field { g.has_ability(id, ab) } else { def.abilities.contains(ab) })
        .map(|(name, _)| *name)
        .collect();

    let mut obj = json!({
        "id": id.0,
        "name": def.name,
        "mana_cost": def.cost.text,
        "mana_value": def.mana_value(),
        "types": types,
        "subtypes": def.subtypes,
    });
    if !keywords.is_empty() {
        obj["keywords"] = json!(keywords);
    }
    if is_creature {
        let (p, t) = if on_field { g.power_toughness(id) } else { (def.power, def.toughness) };
        obj["power"] = json!(p);
        obj["toughness"] = json!(t);
    }
    if on_field {
        obj["tapped"] = json!(o.tapped);
        if o.sick && is_creature {
            obj["summoning_sick"] = json!(true);
        }
        if o.damage > 0 {
            obj["damage"] = json!(o.damage);
        }
        if !o.counters.is_empty() {
            obj["counters"] = json!(o.counters);
        }
        if o.attacking.is_some() {
            obj["attacking"] = json!(true);
        }
        if !o.blocking.is_empty() {
            obj["blocking_ids"] = json!(o.blocking.iter().map(|a| a.0).collect::<Vec<_>>());
        }
    }
    obj
}

fn stack_item_json(g: &Game, seat: Seat, item: &StackItem) -> Value {
    let o = g.obj(item.card);
    let def = item.def.as_deref().unwrap_or(&o.def);
    let targets: Vec<Value> = item
        .targets
        .iter()
        .flatten()
        .map(|t| match t {
            Target::Player(p) => json!({ "kind": "player", "who": if *p == seat { "you" } else { "opponent" } }),
            Target::Obj(id) => json!({ "kind": "obj", "id": id.0, "name": g.obj(*id).def.name }),
        })
        .collect();
    json!({
        "card_id": item.card.0,
        "name": def.name,
        "controller": if item.controller == seat { "you" } else { "opponent" },
        "is_ability": item.ability.is_some() || item.delayed.is_some(),
        "targets": targets,
        "modes": item.modes,
    })
}

fn query_kind_name(q: &Query) -> &'static str {
    match q {
        Query::Priority { .. } => "priority",
        Query::PlayMana { .. } => "play_mana",
        Query::Target { .. } => "target",
        Query::Attackers { .. } => "attackers",
        Query::Blockers { .. } => "blockers",
        Query::BlockTarget { .. } => "block_target",
        Query::Cards { .. } => "cards",
        Query::MultiAmount { .. } => "multi_amount",
        Query::ChooseAbility { .. } => "choose_ability",
        Query::PickTrigger { .. } => "pick_trigger",
        Query::YesNo { .. } => "yes_no",
        Query::ChooseMode { .. } => "choose_mode",
        Query::Amount { .. } => "amount",
        Query::Choice { .. } => "choice",
    }
}

fn query_prompt_text(g: &Game, _seat: Seat, q: &Query) -> String {
    match q {
        Query::Priority { .. } => "Play a land, cast a spell, activate an ability, or pass priority.".to_string(),
        Query::PlayMana { cost, .. } => format!("Pay mana cost {cost}."),
        Query::Target { required, .. } => {
            if *required {
                "Choose a required target.".to_string()
            } else {
                "Choose a target or decline.".to_string()
            }
        }
        Query::Attackers { .. } => "Declare an attacking creature, or finish declaring attackers.".to_string(),
        Query::Blockers { .. } => "Choose a creature to block with, or finish declaring blockers.".to_string(),
        Query::BlockTarget { .. } => "Choose which attacking creature to block.".to_string(),
        Query::Cards { required, .. } => {
            if *required {
                "Choose a card.".to_string()
            } else {
                "Choose a card or decline.".to_string()
            }
        }
        Query::MultiAmount { min, max, .. } => format!("Distribute amounts between {min} and {max}."),
        Query::ChooseAbility { source, .. } => format!("Choose an ability of {} to activate.", g.obj(*source).def.name),
        Query::PickTrigger { count, .. } => format!("Choose which of {count} waiting triggers goes on the stack next."),
        Query::YesNo { message } => message.clone(),
        Query::ChooseMode { .. } => "Choose a mode for the modal spell or ability.".to_string(),
        Query::Amount { min, max } => format!("Choose an integer amount between {min} and {max}."),
        Query::Choice { message, .. } => message.clone(),
    }
}

fn candidate_json(g: &Game, seat: Seat, q: &Query, a: &Answer, index: usize, is_quick: bool) -> Value {
    let id = format!("c{index}");
    let (kind, label, extra_card, extra_obj_id) = match a {
        Answer::Bool(false) => {
            let lbl = match q {
                Query::Priority { .. } => "Pass priority".to_string(),
                Query::Attackers { .. } => "Done selecting attackers".to_string(),
                Query::Blockers { .. } => "Done selecting blockers".to_string(),
                Query::YesNo { .. } => "No".to_string(),
                _ => "Decline / Done".to_string(),
            };
            ("pass_or_no", lbl, None, None)
        }
        Answer::Bool(true) => ("yes", "Yes".to_string(), None, None),
        Answer::Player(p) => {
            let who = if *p == seat { "you" } else { "opponent" };
            let life = g.player(*p).life;
            ("player", format!("Target player: {who} (life {life})"), None, None)
        }
        Answer::Obj(oid) => {
            let o = g.obj(*oid);
            let owner = if o.controller == seat { "your" } else { "opponent's" };
            let name = o.def.name.clone();
            let cost = if o.def.cost.text.is_empty() { String::new() } else { format!(" {}", o.def.cost.text) };
            let stats = if o.zone == Zone::Battlefield && g.has_type(*oid, CardType::Creature) {
                let (p, t) = g.power_toughness(*oid);
                format!(" [{p}/{t}]")
            } else {
                String::new()
            };
            let lbl = match q {
                Query::Priority { .. } => {
                    if o.zone == Zone::Hand && o.def.types.contains(&CardType::Land) {
                        format!("Play land: {name}")
                    } else if o.zone == Zone::Hand || o.zone == Zone::Exile || o.zone == Zone::Graveyard {
                        format!("Cast {name}{cost} (from {:?})", o.zone)
                    } else {
                        format!("Activate {owner} {name}{stats} (id {})", oid.0)
                    }
                }
                Query::Attackers { .. } => format!("Attack with {name}{stats} (id {})", oid.0),
                Query::Blockers { .. } => format!("Block with {name}{stats} (id {})", oid.0),
                Query::BlockTarget { .. } => format!("Block attacker {name}{stats} (id {})", oid.0),
                Query::Target { .. } => format!("Target {owner} {name}{stats} ({:?}, id {})", o.zone, oid.0),
                Query::Cards { .. } => format!("Choose card: {name}{cost} (id {})", oid.0),
                _ => format!("Choose {owner} {name}{stats} (id {})", oid.0),
            };
            ("obj", lbl, Some(name), Some(oid.0))
        }
        Answer::Ability(oid, ab_idx) => {
            let o = g.obj(*oid);
            let name = o.def.name.clone();
            ("ability", format!("Activate ability #{ab_idx} of {name} (id {})", oid.0), Some(name), Some(oid.0))
        }
        Answer::Trigger(t_idx) => {
            let lbl = if let Query::PickTrigger { triggers, .. } = q {
                if let Some((src, text)) = triggers.get(*t_idx) {
                    format!("Stack trigger #{t_idx} from {}: {text}", g.obj(*src).def.name)
                } else {
                    format!("Stack trigger #{t_idx}")
                }
            } else {
                format!("Stack trigger #{t_idx}")
            };
            ("trigger", lbl, None, None)
        }
        Answer::Mode(m) => ("mode", format!("Choose mode #{m}"), None, None),
        Answer::ModeDone => ("mode_done", "Finish choosing modes".to_string(), None, None),
        Answer::ModeCancel => ("mode_cancel", "Cancel spell".to_string(), None, None),
        Answer::Int(n) => ("int", format!("Choose value {n}"), None, None),
        Answer::Text(s) => ("text", format!("Choose {s}"), None, None),
        Answer::ManaType(s) => ("mana_type", format!("Choose mana color {s}"), None, None),
        Answer::Unknown => ("unknown", "Default / Done".to_string(), None, None),
        Answer::Stop => ("stop", "Stop".to_string(), None, None),
    };
    let mut c = json!({
        "index": index,
        "id": id,
        "kind": kind,
        "label": label,
        "is_quick": is_quick,
    });
    if let Some(card) = extra_card {
        c["card"] = json!(card);
    }
    if let Some(oid) = extra_obj_id {
        c["obj_id"] = json!(oid);
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::Hidden;
    use crate::duel::{new_duel, Deck};
    use crate::search::{candidates, Config, Table};
    use std::net::TcpListener;

    fn sample_decks() -> [Deck; 2] {
        let d1 = Deck::parse("red", "20 Mountain\n20 Monastery Swiftspear\n20 Lightning Strike\n");
        let d2 = Deck::parse("white", "20 Plains\n20 Savannah Lions\n20 Serra Angel\n");
        [d1, d2]
    }

    /// Resampling opponent hand and both libraries must never change the serialized external JSON request.
    #[test]
    fn external_state_shows_nothing_the_seat_cant_see() {
        let db = playmat_engine::cards();
        let decks = sample_decks();
        let game = new_duel(&db, [&decks[0], &decks[1]], 42, 0).expect("duel");
        let mut hidden = Hidden::new(0);
        hidden.observe(&game);
        let root = crate::decision::Root {
            snapshot: game.clone(),
            since: Vec::new(),
            hidden: Some(hidden),
            playout: 2,
            rollout: None,
            seat: 0,
            playbook: None,
        };
        let playable: Vec<ObjId> = game.player(0).hand.clone();
        let q = Query::Priority { playable };
        let quick = Answer::Bool(false);
        let cands = candidates(&game, 0, &q, &quick, 12);
        let cfg = Config::default();

        let g1 = root.sample(111);
        let g2 = root.sample(999);
        let d1 =
            Decision { game: &g1, seat: 0, query: &q, quick: &quick, candidates: &cands, config: &cfg, root: None };
        let d2 =
            Decision { game: &g2, seat: 0, query: &q, quick: &quick, candidates: &cands, config: &cfg, root: None };
        let j1 = serde_json::to_string(&build_request(&d1, true)).unwrap();
        let j2 = serde_json::to_string(&build_request(&d2, true)).unwrap();
        assert_eq!(j1, j2, "external request leaked hidden hand/library information");
    }

    #[test]
    fn parses_numeric_and_string_choice_ids() {
        assert_eq!(parse_choice_index(&json!({"choice": 2}), 4), Some(2));
        assert_eq!(parse_choice_index(&json!({"choice": "c1"}), 4), Some(1));
        assert_eq!(parse_choice_index(&json!({"choice": "C3"}), 4), Some(3));
        assert_eq!(parse_choice_index(&json!({"choice": "c9"}), 4), None);
        assert_eq!(parse_choice_index(&json!({"choice": 4}), 4), None);
    }

    #[test]
    fn external_policy_plays_via_tcp_loopback() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        let addr = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept");
            let read_s = stream.try_clone().unwrap();
            let mut reader = BufReader::new(read_s);
            let mut writer = BufWriter::new(stream);
            let mut line = String::new();
            let mut count = 0usize;
            while reader.read_line(&mut line).unwrap_or(0) > 0 {
                let req: Value = serde_json::from_str(line.trim()).expect("valid request json");
                assert_eq!(req["protocol"], PROTOCOL_VERSION);
                assert!(req["state"]["opponent"].get("hand").is_none());
                let k = req["candidates"].as_array().unwrap().len();
                let pick = (k - 1).min(1);
                let mut probs = vec![0.0f64; k];
                probs[pick] = 1.0;
                let resp = json!({
                    "choice": format!("c{pick}"),
                    "probabilities": probs,
                    "value": 0.65,
                    "nodes": 1
                });
                writeln!(writer, "{resp}").unwrap();
                writer.flush().unwrap();
                line.clear();
                count += 1;
            }
            count
        });

        let db = playmat_engine::cards();
        let decks = sample_decks();
        let mut game = new_duel(&db, [&decks[0], &decks[1]], 7, 0).expect("duel");
        let cfg = Config {
            policy: crate::decision::PolicyKind::External,
            external: Some(format!("tcp://{addr}")),
            hide: true,
            ..Config::default()
        };
        let mut table = Table::new(vec![Some(cfg), None]);
        playmat_engine::rules::play_from_priority(&mut game, &mut table, 6);
        drop(table);
        let handled = server.join().expect("server thread");
        assert!(handled > 0, "expected external TCP server to answer at least one decision");
    }
}
