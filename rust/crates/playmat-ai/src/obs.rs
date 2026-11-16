//! What a learned policy sees (#390 Track B, L0): one seat's observation of a game as numbers, and each candidate
//! answer as numbers, for the network in `net` and the trainer in `rust/rl`.
//!
//! An observation has three parts:
//!
//! - global features (`GLOB_F`): life, cards in each zone, lands, the step, whose turn, what is being asked, and the
//!   fitted score's features (`fitted::features`);
//! - entities, one per object the seat can see (`ENT_F` features each, plus a hashed card name): the stack, both
//!   battlefields and the seat's own hand, in that order, at most `MAX_ENT`;
//! - actions, one per candidate answer (`ACT_F` features plus a hashed card name): what kind of answer it is, and the
//!   object it names as an entity would be (a card in a graveyard or a library too, when the answer names one).
//!
//! `observe` never reads what the seat can't know: the opponent's hand cards (only their count), any library's order
//! or contents, the random number generator. `tests/self_play.rs` checks that by swapping hidden cards. The critic
//! used while training may see more: `hidden_entities` lists the opponent's hand, for training only.

use crate::fitted;
use playmat_engine::card::{AbilityDef, CardType, Color};
use playmat_engine::decide::{Answer, Query};
use playmat_engine::state::{Game, ObjId, Seat, Step, Zone};

/// Features per entity
pub const ENT_F: usize = 49;
/// Global features
pub const GLOB_F: usize = 17 + 22 + 13 + 13;
/// Features per action
pub const ACT_F: usize = 16 + ENT_F;
/// Most entities in one observation (the rest are dropped, the stack and permanents first kept)
pub const MAX_ENT: usize = 64;
/// Hash buckets for card names (0: no card)
pub const BUCKETS: u32 = 4096;

/// One seat's view of a game at a question
#[derive(Clone, Debug, Default)]
pub struct Obs {
    pub glob: Vec<f32>,
    /// `ENT_F` features per entity, flat
    pub ents: Vec<f32>,
    pub ent_ids: Vec<u32>,
    /// `ACT_F` features per action, flat
    pub acts: Vec<f32>,
    pub act_ids: Vec<u32>,
}

impl Obs {
    pub fn n_ents(&self) -> usize {
        self.ent_ids.len()
    }
    pub fn n_acts(&self) -> usize {
        self.act_ids.len()
    }
}

/// A card name's bucket: FNV-1a of the name, 1..BUCKETS
pub fn card_id(name: &str) -> u32 {
    let h = name.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ b as u64).wrapping_mul(0x0100_0000_01b3));
    1 + (h % (BUCKETS as u64 - 1)) as u32
}

const KEYWORDS: [AbilityDef; 13] = [
    AbilityDef::FlyingAbility,
    AbilityDef::TrampleAbility,
    AbilityDef::DeathtouchAbility,
    AbilityDef::LifelinkAbility,
    AbilityDef::FirstStrikeAbility,
    AbilityDef::DoubleStrikeAbility,
    AbilityDef::HasteAbility,
    AbilityDef::VigilanceAbility,
    AbilityDef::ReachAbility,
    AbilityDef::MenaceAbility,
    AbilityDef::DefenderAbility,
    AbilityDef::HexproofAbility,
    AbilityDef::IndestructibleAbility,
];

/// An object's features from `seat`'s side (`hidden`: a card the seat can't see, for the critic only)
pub fn entity(game: &Game, seat: Seat, id: ObjId, playable: &[ObjId], hidden: bool) -> [f32; ENT_F] {
    let mut f = [0f32; ENT_F];
    let o = game.obj(id);
    let on_field = o.zone == Zone::Battlefield;
    f[0] = (o.controller == seat) as u8 as f32;
    match o.zone {
        Zone::Hand => f[1] = 1.0,
        Zone::Battlefield => f[2] = 1.0,
        Zone::Stack => f[3] = 1.0,
        Zone::Graveyard => f[4] = 1.0,
        _ => f[5] = 1.0,
    }
    f[6] = hidden as u8 as f32;
    let def = &o.def;
    let has_t = |t: CardType| if on_field { game.has_type(id, t) } else { def.types.contains(&t) };
    for (i, t) in [
        CardType::Artifact,
        CardType::Creature,
        CardType::Enchantment,
        CardType::Instant,
        CardType::Land,
        CardType::Planeswalker,
        CardType::Sorcery,
        CardType::Battle,
    ]
    .into_iter()
    .enumerate()
    {
        f[7 + i] = has_t(t) as u8 as f32;
    }
    f[15] = o.token as u8 as f32;
    f[16] = def.mana_value() as f32 / 5.0;
    let (p, t) =
        if on_field && has_t(CardType::Creature) { game.power_toughness(id) } else { (def.power, def.toughness) };
    f[17] = p as f32 / 5.0;
    f[18] = t as f32 / 5.0;
    f[19] = o.damage as f32 / 5.0;
    f[20] = o.tapped as u8 as f32;
    f[21] = (on_field && o.sick) as u8 as f32;
    f[22] = o.attacking.is_some() as u8 as f32;
    f[23] = !o.blocking.is_empty() as u8 as f32;
    f[24] = o.blocked as u8 as f32;
    for (name, n) in &o.counters {
        match name.as_str() {
            "+1/+1" => f[25] += *n as f32 / 3.0,
            "loyalty" => f[26] += *n as f32 / 5.0,
            _ => f[27] += *n as f32 / 3.0,
        }
    }
    let colors = if on_field { game.colors(id) } else { def.colors.clone() };
    for c in colors {
        let i = match c {
            Color::W => 0,
            Color::U => 1,
            Color::B => 2,
            Color::R => 3,
            Color::G => 4,
        };
        f[28 + i] = 1.0;
    }
    for (i, k) in KEYWORDS.iter().enumerate() {
        f[33 + i] = if on_field { game.has_ability(id, k) } else { def.abilities.contains(k) } as u8 as f32;
    }
    f[46] = def.abilities.contains(&AbilityDef::FlashAbility) as u8 as f32;
    f[47] = playable.contains(&id) as u8 as f32;
    f[48] = def.subtypes.iter().any(|s| s == "Aura" || s == "Equipment") as u8 as f32;
    f
}

fn step_index(s: Option<Step>) -> usize {
    match s {
        Some(Step::Untap) => 0,
        Some(Step::Upkeep) => 1,
        Some(Step::Draw) => 2,
        Some(Step::PrecombatMain) => 3,
        Some(Step::BeginCombat) => 4,
        Some(Step::DeclareAttackers) => 5,
        Some(Step::DeclareBlockers) => 6,
        Some(Step::FirstCombatDamage) => 7,
        Some(Step::CombatDamage) => 8,
        Some(Step::EndCombat) => 9,
        Some(Step::PostcombatMain) => 10,
        Some(Step::EndTurn) => 11,
        Some(Step::Cleanup) | None => 12,
    }
}

/// The question's kind, 0..13
pub fn query_index(q: &Query) -> usize {
    match q {
        Query::Priority { .. } => 0,
        Query::PlayMana { .. } => 1,
        Query::Target { .. } => 2,
        Query::Attackers { .. } => 3,
        Query::Blockers { .. } => 4,
        Query::BlockTarget { .. } => 5,
        Query::Cards { .. } => 6,
        Query::MultiAmount { .. } => 7,
        Query::ChooseAbility { .. } => 8,
        Query::PickTrigger { .. } => 9,
        Query::YesNo { .. } => 10,
        Query::ChooseMode { .. } | Query::Amount { .. } => 11,
        Query::Choice { .. } => 12,
    }
}

/// The global features
pub fn global(game: &Game, seat: Seat, q: &Query) -> Vec<f32> {
    let mut g: Vec<f32> = fitted::features(game, seat).iter().map(|v| *v as f32).collect();
    let opp = game.opponent(seat);
    // the fitted score's hand feature compares mana values in both hands: the opponent's is hidden, so only the
    // seat's own here
    g[16] = game.player(seat).hand.iter().map(|id| game.obj(*id).def.mana_value() as f32).sum::<f32>() / 10.0;
    let lands = |s: Seat, untapped: bool| {
        game.battlefield
            .iter()
            .filter(|id| {
                let o = game.obj(**id);
                o.controller == s && game.has_type(**id, CardType::Land) && (!untapped || !o.tapped)
            })
            .count() as f32
    };
    let poison = |s: Seat| game.player(s).counters.iter().filter(|(n, _)| n == "poison").map(|(_, v)| *v).sum::<i32>();
    let (me, them) = (game.player(seat), game.player(opp));
    let pool = |s: Seat| game.player(s).mana_pool.total() as f32;
    g.extend([
        game.turn as f32 / 10.0,
        me.life as f32 / 20.0,
        them.life as f32 / 20.0,
        me.hand.len() as f32 / 7.0,
        them.hand.len() as f32 / 7.0,
        me.library.len() as f32 / 40.0,
        them.library.len() as f32 / 40.0,
        me.graveyard.len() as f32 / 10.0,
        them.graveyard.len() as f32 / 10.0,
        lands(seat, true) / 5.0,
        lands(opp, true) / 5.0,
        lands(seat, false) / 5.0,
        lands(opp, false) / 5.0,
        me.lands_played as f32,
        (game.active == Some(seat)) as u8 as f32,
        (game.priority == Some(seat)) as u8 as f32,
        game.stack.len() as f32 / 3.0,
        game.stack.last().is_some_and(|s| s.controller != seat) as u8 as f32,
        poison(seat) as f32 / 10.0,
        poison(opp) as f32 / 10.0,
        pool(seat) / 3.0,
        game.exile.len() as f32 / 10.0,
    ]);
    let mut step = [0f32; 13];
    step[step_index(game.step)] = 1.0;
    g.extend(step);
    let mut kind = [0f32; 13];
    kind[query_index(q)] = 1.0;
    g.extend(kind);
    debug_assert_eq!(g.len(), GLOB_F);
    g
}

/// The objects the seat sees, in order: the stack, permanents (its own first), its hand; at most `MAX_ENT`
pub fn visible(game: &Game, seat: Seat) -> Vec<ObjId> {
    let mut out: Vec<ObjId> = game.stack.iter().rev().map(|s| s.card).collect();
    out.extend(game.battlefield.iter().filter(|id| game.obj(**id).controller == seat));
    out.extend(game.battlefield.iter().filter(|id| game.obj(**id).controller != seat));
    out.extend(game.player(seat).hand.iter());
    out.dedup();
    out.truncate(MAX_ENT);
    out
}

/// What `seat` can't see that the critic may: the opponent's hand
pub fn hidden_entities(game: &Game, seat: Seat) -> (Vec<f32>, Vec<u32>) {
    let opp = game.opponent(seat);
    let mut f = Vec::new();
    let mut ids = Vec::new();
    for id in game.player(opp).hand.iter().take(MAX_ENT) {
        f.extend(entity(game, seat, *id, &[], true));
        ids.push(card_id(&game.obj(*id).def.name));
    }
    (f, ids)
}

fn named(a: &Answer) -> Option<ObjId> {
    match a {
        Answer::Obj(id) | Answer::Ability(id, _) => Some(*id),
        _ => None,
    }
}

/// One candidate's features (`quick`: it's the quick answers' pick)
pub fn action(game: &Game, seat: Seat, q: &Query, a: &Answer, quick: bool, playable: &[ObjId]) -> ([f32; ACT_F], u32) {
    let mut f = [0f32; ACT_F];
    let kind = match a {
        Answer::Bool(false) => 0,
        Answer::Bool(true) => 1,
        Answer::Obj(_) => 2,
        Answer::Ability(..) => 3,
        Answer::Player(_) => 4,
        Answer::Int(_) => 5,
        Answer::Text(_) | Answer::ManaType(_) => 6,
        Answer::Mode(_) | Answer::ModeDone | Answer::ModeCancel => 7,
        Answer::Trigger(_) => 8,
        _ => 9,
    };
    f[kind] = 1.0;
    if let Answer::Player(p) = a {
        f[10] = (*p == seat) as u8 as f32;
        f[11] = (*p != seat) as u8 as f32;
    }
    f[12] = match a {
        Answer::Int(n) => *n as f32 / 5.0,
        Answer::Mode(n) | Answer::Trigger(n) => *n as f32 / 3.0,
        _ => 0.0,
    };
    if let Answer::Ability(_, i) = a {
        f[13] = *i as f32 / 3.0;
    }
    f[14] = quick as u8 as f32;
    // a creature answered to the blockers question is a blocker; to the attackers one, an attacker toggled
    f[15] = matches!(q, Query::Blockers { .. } | Query::Attackers { .. }) as u8 as f32;
    let mut id = 0;
    if let Some(o) = named(a) {
        if (o.0 as usize) < game.objs.len() {
            let hidden = game.obj(o).zone == Zone::Hand && game.obj(o).owner != seat;
            f[16..].copy_from_slice(&entity(game, seat, o, playable, false));
            if !hidden {
                id = card_id(&game.obj(o).def.name);
            } else {
                f[16..].fill(0.0);
            }
        }
    }
    (f, id)
}

/// The seat's observation at question `q`, with its candidate answers (`candidates[0]` the quick one)
pub fn observe(game: &Game, seat: Seat, q: &Query, candidates: &[Answer]) -> Obs {
    let playable: &[ObjId] = match q {
        Query::Priority { playable } => playable,
        _ => &[],
    };
    let mut o = Obs { glob: global(game, seat, q), ..Obs::default() };
    for id in visible(game, seat) {
        o.ents.extend(entity(game, seat, id, playable, false));
        o.ent_ids.push(card_id(&game.obj(id).def.name));
    }
    for (i, a) in candidates.iter().enumerate() {
        let (f, id) = action(game, seat, q, a, i == 0, playable);
        o.acts.extend(f);
        o.act_ids.push(id);
    }
    o
}
