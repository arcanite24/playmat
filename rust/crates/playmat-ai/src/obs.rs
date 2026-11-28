//! What a learned policy sees (#390 Track B, Obs v2): one seat's observation of a game as numbers, sparse
//! relational edges between entities, and each candidate answer linked to its entity token.
//!
//! An observation has three parts:
//!
//! - global features (`GLOB_F = 81`): `0..65` match v1 (`fitted::features`, life, zones, lands, step, query kind) so
//!   v1/Wave A `.pmnet` models can slice `&glob[..65]` cleanly; `65..81` add per-color untapped mana sources for both
//!   players `(W,U,B,R,G,C)`, incoming/outgoing unblocked combat damage vs life, and top-of-stack targeting/spell flags.
//! - entities (`ENT_F = 49` each, plus card bucket and sparse relational edge `(ent_rel_idx, ent_rel_type)`), in order:
//!   distinct stack items (top-first, never deduplicated against permanents), own battlefield, opponent battlefield,
//!   own hand, playable/face-up exile, and top-K graveyards (own then opponent), capped at `MAX_ENT = 80`.
//! - actions (`ACT_F = 65` each, plus card bucket and `act_ent_idx` pointing to the entity token the action names).
//!
//! `observe` never reads what the seat can't know: opponent's hidden hand cards, any library's order or contents, or
//! face-down exile the seat cannot play. `tests/self_play.rs` verifies zero leakage across resampled hidden states.

use crate::fitted;
use playmat_engine::card::{AbilityDef, CardType, Color, EffectDef};
use playmat_engine::decide::{Answer, Query};
use playmat_engine::state::{Game, ObjId, Seat, StackItem, Step, Target, Zone};

/// Features per entity
pub const ENT_F: usize = 49;
/// Legacy v1 global feature width (prefix of `GLOB_F` for backward-compatible `.pmnet` loading)
pub const GLOB_F_V1: usize = 17 + 22 + 13 + 13;
/// Legacy v2 global feature width (prefix of `GLOB_F` for backward-compatible `.pmnet` loading)
pub const GLOB_F_V2: usize = GLOB_F_V1 + 16;
/// Legacy v3 global feature width (prefix of `GLOB_F` for backward-compatible `.pmnet` loading)
pub const GLOB_F_V3: usize = GLOB_F_V2 + 8;
/// Global features (v3 89 + 8 public pass/action-history & tempo features)
pub const GLOB_F: usize = GLOB_F_V3 + 8;
/// Width of v3 structured public-belief summary vector (`belief_meta.f16`)
pub const BELIEF_META_F: usize = 8;
/// Features per action
pub const ACT_F: usize = 16 + ENT_F;
/// Most entities in one observation (stack, battlefields, own hand, playable exile, top-K graveyards)
pub const MAX_ENT: usize = 80;
/// Hash buckets for card names (0: no card)
pub const BUCKETS: u32 = 4096;
/// Sentinel index when an entity has no relational target or an action names no visible entity
pub const NO_ENT: u8 = 255;
/// Number of directed relational edge types (1: stack->target, 2: blocker->attacker, 3: aura/equip->attached, 4: stack->source)
pub const NUM_REL_TYPES: usize = 4;

/// Top-K most recent graveyard cards per player included as entities
const GY_TOP_K: usize = 6;

/// One seat's view of a game at a question
#[derive(Clone, Debug, Default)]
pub struct Obs {
    pub glob: Vec<f32>,
    /// `ENT_F` features per entity, flat
    pub ents: Vec<f32>,
    pub ent_ids: Vec<u32>,
    /// Primary relational edge per entity: target entity index (`0..n_ents`, or `NO_ENT`)
    pub ent_rel_idx: Vec<u8>,
    /// Primary relational edge type per entity (`0` none, `1` stack->target, `2` blocker->attacker,
    /// `3` aura/equip->attached, `4` stack->source)
    pub ent_rel_type: Vec<u8>,
    /// `ACT_F` features per action, flat
    pub acts: Vec<f32>,
    pub act_ids: Vec<u32>,
    /// Entity index in `0..n_ents` named by each candidate action (or `NO_ENT`)
    pub act_ent_idx: Vec<u8>,
}

impl Obs {
    pub fn n_ents(&self) -> usize {
        self.ent_ids.len()
    }
    pub fn n_acts(&self) -> usize {
        self.act_ids.len()
    }

    /// 128-bit content hash of all observable features for exact neural-evaluation memoization.
    pub fn hash128(&self) -> (u64, u64) {
        let mut h1 = 0xcbf2_9ce4_8422_2325u64;
        let mut h2 = 0x9e37_79b9_7f4a_7c15u64;
        let mut mix = |v: u64| {
            h1 ^= v;
            h1 = h1.wrapping_mul(0x0100_0000_01b3);
            h2 = h2.wrapping_add(v ^ (v >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        };
        mix(self.glob.len() as u64);
        mix(self.ent_ids.len() as u64);
        mix(self.act_ids.len() as u64);
        for &x in &self.glob {
            mix(if x == 0.0 { 0 } else { x.to_bits() as u64 });
        }
        for &x in &self.ents {
            mix(if x == 0.0 { 0 } else { x.to_bits() as u64 });
        }
        for &id in &self.ent_ids {
            mix(id as u64);
        }
        for (&ri, &rt) in self.ent_rel_idx.iter().zip(&self.ent_rel_type) {
            mix(((ri as u64) << 8) | (rt as u64));
        }
        for &x in &self.acts {
            mix(if x == 0.0 { 0 } else { x.to_bits() as u64 });
        }
        for &id in &self.act_ids {
            mix(id as u64);
        }
        for &ae in &self.act_ent_idx {
            mix(ae as u64);
        }
        (h1, h2)
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

const KEYWORD_COUNTERS: [&str; 13] = [
    "flying",
    "trample",
    "deathtouch",
    "lifelink",
    "first strike",
    "double strike",
    "haste",
    "vigilance",
    "reach",
    "menace",
    "",
    "hexproof",
    "indestructible",
];

fn keyword_index(a: &AbilityDef) -> Option<usize> {
    KEYWORDS.iter().position(|k| k == a)
}

/// Pre-scanned continuous-effect summary for a single `Game` snapshot so `entity` / `global` skip redundant
/// `O(|battlefield|)` layer scans when no static grants, ability removals, color overrides, or P/T statics exist.
#[derive(Clone, Copy)]
struct BoardLayerFlags {
    may_grant_mask: u16,
    may_lose: bool,
    simple_colors: bool,
    simple_pt_board: bool,
}

impl BoardLayerFlags {
    fn scan(game: &Game) -> Self {
        let mut flags = Self {
            may_grant_mask: 0,
            may_lose: !game.lost_until_eot.is_empty() || !game.lost_until_turn_of.is_empty(),
            simple_colors: game.color_overrides.is_empty(),
            simple_pt_board: game.base_pt_until_eot.is_empty() && game.base_pt_sets.is_empty(),
        };
        if !game.card_statics.is_empty() {
            flags.may_grant_mask = 0x1fff;
            flags.may_lose = true;
            flags.simple_pt_board = false;
        }
        fn scan_effect(e: &EffectDef, flags: &mut BoardLayerFlags) {
            match e {
                EffectDef::ConditionalContinuousEffect { effect, .. } => scan_effect(effect, flags),
                EffectDef::CardStatic { .. } => {
                    flags.may_grant_mask = 0x1fff;
                    flags.may_lose = true;
                    flags.simple_pt_board = false;
                }
                EffectDef::LeylineOfTheGuildpactEffect => {
                    flags.simple_colors = false;
                }
                EffectDef::LoseAllAbilitiesAttachedEffect
                | EffectDef::LoseAbilityAttachedEffect { .. }
                | EffectDef::CreaturesCantGetOrHaveAbilityEffect { .. } => {
                    flags.may_lose = true;
                }
                EffectDef::GainAbilityAllEffect { ability, .. }
                | EffectDef::GainAbilityControlledEffect { ability, .. }
                | EffectDef::GainAbilitySourceEffect { ability, .. }
                | EffectDef::GainAbilityAttachedEffect { ability } => {
                    if let Some(ki) = keyword_index(ability) {
                        flags.may_grant_mask |= 1 << ki;
                    }
                    if let AbilityDef::SimpleStaticAbility { effects, .. } = &**ability {
                        flags.simple_pt_board = false;
                        for ge in effects {
                            scan_effect(ge, flags);
                        }
                    }
                }
                _ => {
                    flags.simple_pt_board = false;
                }
            }
        }
        for g in game.grants.iter() {
            if let Some(ki) = keyword_index(&g.ability) {
                flags.may_grant_mask |= 1 << ki;
            }
            if let AbilityDef::SimpleStaticAbility { effects, .. } = &g.ability {
                for e in effects {
                    scan_effect(e, &mut flags);
                }
            }
        }
        for &src in game.battlefield.iter().chain(game.command.iter()) {
            let s = game.obj(src);
            if s.lost_abilities.is_some() {
                flags.may_lose = true;
            }
            if !s.counters.is_empty() && s.def.has_subtype("Equipment") && s.counter("hone") != 0 {
                flags.simple_pt_board = false;
            }
            for a in &s.def.abilities {
                if let AbilityDef::SimpleStaticAbility { effects, .. } = a {
                    for e in effects {
                        scan_effect(e, &mut flags);
                    }
                }
            }
        }
        flags
    }

    #[inline]
    fn has_keyword(&self, game: &Game, id: ObjId, o: &playmat_engine::state::Obj, ki: usize, k: &AbilityDef) -> bool {
        if !self.may_lose && o.lost_abilities.is_none() {
            if o.has(k) {
                return true;
            }
            if o.lost_all == 0 {
                let kc = KEYWORD_COUNTERS[ki];
                if (!o.counters.is_empty() && !kc.is_empty() && o.counter(kc) > 0) || (ki == 9 && o.suspected()) {
                    return true;
                }
            }
            if (self.may_grant_mask & (1 << ki)) == 0 {
                return false;
            }
        }
        game.has_ability(id, k)
    }

    #[inline]
    fn creature_pt(&self, game: &Game, id: ObjId, o: &playmat_engine::state::Obj) -> (i32, i32) {
        if self.simple_pt_board && !o.phased_out && o.becomes_base.is_none() && o.base_pt.is_none() {
            let counters = if o.counters.is_empty() { 0 } else { o.counter("+1/+1") - o.counter("-1/-1") };
            let (mut p, mut t) = (o.def.power + counters, o.def.toughness + counters);
            if !game.boosts.is_empty() {
                let mut switched = false;
                for b in game.boosts.iter() {
                    if b.target == id && b.zcc == o.zcc {
                        p += b.power;
                        t += b.toughness;
                        switched ^= b.switch;
                    }
                }
                if switched {
                    (p, t) = (t, p);
                }
            }
            return (p, t);
        }
        game.power_toughness(id)
    }
}

/// An object's features from `seat`'s side (`hidden`: a card the seat can't see, for the critic only)
pub fn entity(game: &Game, seat: Seat, id: ObjId, playable: &[ObjId], hidden: bool) -> [f32; ENT_F] {
    let flags = BoardLayerFlags::scan(game);
    entity_with_flags(game, seat, id, playable, hidden, &flags)
}

fn entity_with_flags(
    game: &Game,
    seat: Seat,
    id: ObjId,
    playable: &[ObjId],
    hidden: bool,
    flags: &BoardLayerFlags,
) -> [f32; ENT_F] {
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
    let has_t = |t: CardType| if on_field { o.is(t) } else { def.types.contains(&t) };
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
    let (p, t) = if on_field && f[8] != 0.0 { flags.creature_pt(game, id, o) } else { (def.power, def.toughness) };
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
    let mut mark_color = |c: Color| {
        let i = match c {
            Color::W => 0,
            Color::U => 1,
            Color::B => 2,
            Color::R => 3,
            Color::G => 4,
        };
        f[28 + i] = 1.0;
    };
    if on_field && !flags.simple_colors {
        for c in game.colors(id) {
            mark_color(c);
        }
    } else {
        for &c in &def.colors {
            mark_color(c);
        }
    }
    for (i, k) in KEYWORDS.iter().enumerate() {
        f[33 + i] =
            if on_field { flags.has_keyword(game, id, o, i, k) } else { def.abilities.contains(k) } as u8 as f32;
    }
    f[46] = def.abilities.contains(&AbilityDef::FlashAbility) as u8 as f32;
    f[47] = playable.contains(&id) as u8 as f32;
    f[48] = def.subtypes.iter().any(|s| s == "Aura" || s == "Equipment") as u8 as f32;
    f
}

/// Features for a distinct stack item (spell or activated/triggered ability on the stack).
/// Always encodes `Zone::Stack` (`f[3] = 1.0`) and `s.controller == seat` even when `s.card` is on the battlefield.
fn stack_entity(game: &Game, seat: Seat, s: &StackItem) -> ([f32; ENT_F], u32) {
    let mut f = [0f32; ENT_F];
    let o = game.obj(s.card);
    let def = s.def.as_deref().unwrap_or(&o.def);
    f[0] = (s.controller == seat) as u8 as f32;
    f[3] = 1.0; // Zone::Stack
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
        f[7 + i] = def.types.contains(&t) as u8 as f32;
    }
    f[15] = (s.ability.is_some()) as u8 as f32; // distinguish triggered/activated ability vs spell on stack
    f[16] = def.mana_value() as f32 / 5.0;
    f[17] = def.power as f32 / 5.0;
    f[18] = def.toughness as f32 / 5.0;
    for c in &def.colors {
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
        f[33 + i] = def.abilities.contains(k) as u8 as f32;
    }
    f[46] = def.abilities.contains(&AbilityDef::FlashAbility) as u8 as f32;
    f[48] = def.subtypes.iter().any(|sub| sub == "Aura" || sub == "Equipment") as u8 as f32;
    (f, card_id(&def.name))
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

/// Untapped mana sources on the battlefield for `seat` by `(W, U, B, R, G, C)`, scaled by `/ 3.0`.
/// Reads only public battlefield state (zero hidden info leak).
fn untapped_mana_sources(game: &Game, seat: Seat, flags: &BoardLayerFlags) -> [f32; 6] {
    let mut out = [0f32; 6];
    for &id in game.battlefield.iter() {
        let o = game.obj(id);
        if o.controller != seat || o.tapped {
            continue;
        }
        if o.is(CardType::Creature) && o.sick && !flags.has_keyword(game, id, o, 6, &AbilityDef::HasteAbility) {
            continue;
        }
        let mut can = [false; 6];
        for ab in &o.def.abilities {
            if let Some(m) = ab.mana() {
                for ci in 0..5 {
                    if m.colored[ci] > 0 {
                        can[ci] = true;
                    }
                }
                if m.colorless > 0 {
                    can[5] = true;
                }
            }
        }
        for ci in 0..6 {
            if can[ci] {
                out[ci] += 1.0 / 3.0;
            }
        }
    }
    out
}

/// Unblocked attacking creature power controlled by `attacker` divided by `defender` life, capped at 2.0.
fn unblocked_clock(game: &Game, attacker: Seat, defender: Seat, flags: &BoardLayerFlags) -> f32 {
    let mut p = 0i32;
    for &id in game.battlefield.iter() {
        let o = game.obj(id);
        if o.controller == attacker && o.attacking.is_some() && !o.blocked && o.is(CardType::Creature) {
            p += flags.creature_pt(game, id, o).0.max(0);
        }
    }
    let life = game.player(defender).life.max(1) as f32;
    (p as f32 / life).min(2.0)
}

/// The global features (`0..65` v1 compatible prefix, `65..81` v2 tactical additions)
pub fn global(game: &Game, seat: Seat, q: &Query) -> Vec<f32> {
    let flags = BoardLayerFlags::scan(game);
    global_with_flags(game, seat, q, &flags)
}

fn global_with_flags(game: &Game, seat: Seat, q: &Query, flags: &BoardLayerFlags) -> Vec<f32> {
    let mut g = Vec::with_capacity(GLOB_F);
    g.extend(fitted::features(game, seat).iter().map(|v| *v as f32));
    let opp = game.opponent(seat);
    // the fitted score's hand feature compares mana values in both hands: the opponent's is hidden, so only the
    // seat's own here
    g[16] = game.player(seat).hand.iter().map(|id| game.obj(*id).def.mana_value() as f32).sum::<f32>() / 10.0;
    let (mut seat_untapped_n, mut seat_total_n, mut opp_untapped_n, mut opp_total_n) = (0usize, 0usize, 0usize, 0usize);
    for &id in game.battlefield.iter() {
        let o = game.obj(id);
        if o.is(CardType::Land) {
            if o.controller == seat {
                seat_total_n += 1;
                if !o.tapped {
                    seat_untapped_n += 1;
                }
            } else if o.controller == opp {
                opp_total_n += 1;
                if !o.tapped {
                    opp_untapped_n += 1;
                }
            }
        }
    }
    let seat_untapped = seat_untapped_n as f32;
    let seat_total = seat_total_n as f32;
    let opp_untapped = opp_untapped_n as f32;
    let opp_total = opp_total_n as f32;
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
        seat_untapped / 5.0,
        opp_untapped / 5.0,
        seat_total / 5.0,
        opp_total / 5.0,
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
    debug_assert_eq!(g.len(), GLOB_F_V1);

    // Obs v2 additions (indices 65..81)
    g.extend(untapped_mana_sources(game, seat, flags));
    g.extend(untapped_mana_sources(game, opp, flags));
    g.push(unblocked_clock(game, opp, seat, flags));
    g.push(unblocked_clock(game, seat, opp, flags));
    let top_targets_me = game.stack.last().is_some_and(|s| {
        s.targets.iter().flatten().any(|t| match *t {
            Target::Player(p) => p == seat,
            Target::Obj(oid) => (oid.0 as usize) < game.objs.len() && game.obj(oid).controller == seat,
        })
    });
    let top_is_spell = game.stack.last().is_some_and(|s| s.ability.is_none());
    g.push(top_targets_me as u8 as f32);
    g.push(top_is_spell as u8 as f32);
    debug_assert_eq!(g.len(), GLOB_F_V2);

    // Obs v3 opponent archetype / decklist conditioning (indices 81..89):
    // Strictly computed from legally visible opponent cards (battlefield, graveyard, stack, face-up exile)
    // so there is zero hidden-information leakage in Bo1, and ready for open-decklist Bo3 conditioning.
    g.extend(opponent_archetype_features(game, seat));
    debug_assert_eq!(g.len(), GLOB_F_V3);

    // Obs v4 public pass/action-history & tempo features (indices 89..97):
    // Strictly public game state (zero hidden-information leakage).
    let opp_draw_go = (game.active != Some(opp)
        && opp_untapped >= 2.0
        && them.spells_cast_last_turn == 0
        && !them.hand.is_empty()) as u8 as f32;
    let opp_open_frac = if opp_total > 0.0 { opp_untapped / opp_total } else { 0.0 };
    let tempo_life = (((them.life_lost + them.life_lost_last_turn) - (me.life_lost + me.life_lost_last_turn)) as f32 / 10.0).clamp(-2.0, 2.0);
    g.extend([
        (me.spells_cast as f32 / 3.0).min(2.0),
        (them.spells_cast as f32 / 3.0).min(2.0),
        (them.spells_cast_last_turn as f32 / 3.0).min(2.0),
        (them.expended as f32 / 6.0).min(2.0),
        (me.expended as f32 / 6.0).min(2.0),
        opp_draw_go,
        opp_open_frac,
        tempo_life,
    ]);
    debug_assert_eq!(g.len(), GLOB_F);
    g
}

/// 8 opponent archetype & interaction-threat features computed strictly from publicly visible opponent cards:
/// [sweeper, counterspell, face_burn, spot_removal, combat_trick_or_instant, avg_nonland_mv/5, creature_ratio, seen_frac]
pub fn opponent_archetype_features(game: &Game, seat: Seat) -> [f32; 8] {
    let opp = game.opponent(seat);
    let mut sweeper = 0.0f32;
    let mut counter = 0.0f32;
    let mut burn = 0.0f32;
    let mut removal = 0.0f32;
    let mut trick = 0.0f32;
    let mut mv_sum = 0.0f32;
    let mut creatures = 0.0f32;
    let mut nonlands = 0.0f32;
    let mut total_seen = 0.0f32;

    let mut inspect = |id: ObjId| {
        let o = game.obj(id);
        if o.owner != opp || o.token {
            return;
        }
        total_seen += 1.0;
        let def = &o.def;
        if def.types.contains(&CardType::Land) {
            return;
        }
        nonlands += 1.0;
        mv_sum += def.mana_value() as f32;
        if def.types.contains(&CardType::Creature) {
            creatures += 1.0;
        }
        let k = crate::policy::spell_kind(def);
        if k.sweeper {
            sweeper += 1.0;
        }
        if k.removal {
            removal += 1.0;
        }
        if k.face && k.damage.is_some() {
            burn += 1.0;
        }
        if k.pump || (def.types.contains(&CardType::Instant) && !k.removal && !k.face) {
            trick += 1.0;
        }
        if k.counter {
            counter += 1.0;
        }
    };

    for &id in game.battlefield.iter() {
        inspect(id);
    }
    for &id in game.player(opp).graveyard.iter() {
        inspect(id);
    }
    for st in game.stack.iter() {
        if st.controller == opp {
            inspect(st.card);
        }
    }
    for &id in game.exile.iter() {
        if !game.obj(id).exiled_face_down {
            inspect(id);
        }
    }
    let denom = nonlands.max(1.0);
    let opp_total = (total_seen + game.player(opp).hand.len() as f32 + game.player(opp).library.len() as f32).max(1.0);
    [
        (sweeper / 2.0).min(1.0),
        (counter / 3.0).min(1.0),
        (burn / 4.0).min(1.0),
        (removal / 4.0).min(1.0),
        (trick / 3.0).min(1.0),
        (mv_sum / denom / 5.0).min(2.0),
        creatures / denom,
        (total_seen / opp_total).min(1.0),
    ]
}

/// 8 binary ground-truth hidden-hand threat labels written to v4 shards (`belief_meta.f16`) for training-time
/// auxiliary supervision ONLY (never passed as input to inference):
/// [has_counter, has_sweeper, has_spot_removal, has_burn_face, has_combat_trick, has_playable_instant_now, has_creature, has_land]
pub fn belief_meta(game: &Game, seat: Seat, _unseen_hand_count: usize) -> [f32; BELIEF_META_F] {
    let opp = game.opponent(seat);
    let opp_untapped_lands = game
        .battlefield
        .iter()
        .filter(|id| {
            let o = game.obj(**id);
            o.controller == opp && game.has_type(**id, CardType::Land) && !o.tapped
        })
        .count();
    belief_meta_with_lands(game, seat, opp_untapped_lands)
}

/// Fast path for repeated threat scoring over resampled worlds sharing the same battlefield.
pub fn belief_meta_with_lands(game: &Game, seat: Seat, opp_untapped_lands: usize) -> [f32; BELIEF_META_F] {
    let opp = game.opponent(seat);
    let mut out = [0.0f32; BELIEF_META_F];
    for &id in game.player(opp).hand.iter() {
        let def = &game.obj(id).def;
        if def.types.contains(&CardType::Land) {
            out[7] = 1.0;
            continue;
        }
        if def.types.contains(&CardType::Creature) {
            out[6] = 1.0;
        }
        let is_instant = def.types.contains(&CardType::Instant)
            || def.abilities.contains(&AbilityDef::FlashAbility);
        if is_instant && (def.mana_value() as usize) <= opp_untapped_lands {
            out[5] = 1.0;
        }
        let k = crate::policy::spell_kind(def);
        if k.sweeper {
            out[1] = 1.0;
        }
        if k.removal {
            out[2] = 1.0;
        }
        if k.face {
            out[3] = 1.0;
        }
        if k.pump {
            out[4] = 1.0;
        }
        if k.counter {
            out[0] = 1.0;
        }
    }
    out
}

/// The objects the seat sees (legacy helper kept for callers): stack, permanents, own hand, playable exile, top-K GY
pub fn visible(game: &Game, seat: Seat) -> Vec<ObjId> {
    let mut out: Vec<ObjId> = game.stack.iter().rev().map(|s| s.card).collect();
    out.extend(game.battlefield.iter().filter(|id| game.obj(**id).controller == seat));
    out.extend(game.battlefield.iter().filter(|id| game.obj(**id).controller != seat));
    out.extend(game.player(seat).hand.iter());
    out.extend(game.exile.iter().filter(|id| !game.obj(**id).exiled_face_down && game.may_play_now(**id, seat)));
    out.extend(game.player(seat).graveyard.iter().rev().take(GY_TOP_K));
    out.extend(game.player(game.opponent(seat)).graveyard.iter().rev().take(GY_TOP_K));
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

#[inline]
fn action_header(seat: Seat, q: &Query, a: &Answer, quick: bool) -> [f32; ACT_F] {
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
    f
}

/// One candidate's features (`quick`: it's the quick answers' pick)
pub fn action(game: &Game, seat: Seat, q: &Query, a: &Answer, quick: bool, playable: &[ObjId]) -> ([f32; ACT_F], u32) {
    let flags = BoardLayerFlags::scan(game);
    action_with_flags(game, seat, q, a, quick, playable, &flags)
}

fn action_with_flags(
    game: &Game,
    seat: Seat,
    q: &Query,
    a: &Answer,
    quick: bool,
    playable: &[ObjId],
    flags: &BoardLayerFlags,
) -> ([f32; ACT_F], u32) {
    let mut f = action_header(seat, q, a, quick);
    let mut id = 0;
    if let Some(o) = named(a) {
        if (o.0 as usize) < game.objs.len() {
            let hidden = game.obj(o).zone == Zone::Hand && game.obj(o).owner != seat;
            if !hidden {
                f[16..].copy_from_slice(&entity_with_flags(game, seat, o, playable, false, flags));
                id = card_id(&game.obj(o).def.name);
            }
        }
    }
    (f, id)
}

#[derive(Clone, Copy)]
enum EntEntry {
    Stack(usize), // index into game.stack
    Obj(ObjId),
}

/// The seat's observation at question `q`, with its candidate answers (`candidates[0]` the quick one)
pub fn observe(game: &Game, seat: Seat, q: &Query, candidates: &[Answer]) -> Obs {
    let playable: &[ObjId] = match q {
        Query::Priority { playable } => playable,
        _ => &[],
    };
    let flags = BoardLayerFlags::scan(game);
    let opp = game.opponent(seat);
    let mut entries: Vec<EntEntry> = Vec::with_capacity(MAX_ENT);
    // 1. Distinct stack items (top-first, never merged with battlefield permanents)
    for si in (0..game.stack.len()).rev() {
        if entries.len() >= MAX_ENT {
            break;
        }
        entries.push(EntEntry::Stack(si));
    }
    // 2. Battlefield (own then opponent)
    for &id in game.battlefield.iter().filter(|id| game.obj(**id).controller == seat) {
        if entries.len() >= MAX_ENT {
            break;
        }
        entries.push(EntEntry::Obj(id));
    }
    for &id in game.battlefield.iter().filter(|id| game.obj(**id).controller != seat) {
        if entries.len() >= MAX_ENT {
            break;
        }
        entries.push(EntEntry::Obj(id));
    }
    // 3. Own hand
    for &id in &game.player(seat).hand {
        if entries.len() >= MAX_ENT {
            break;
        }
        entries.push(EntEntry::Obj(id));
    }
    // 4. Playable face-up exile (Impulse draw, Foretell, Plot, Adventure, Escape-like exile)
    for &id in game.exile.iter() {
        if entries.len() >= MAX_ENT {
            break;
        }
        let obj = game.obj(id);
        if !obj.exiled_face_down && game.may_play_now(id, seat) {
            entries.push(EntEntry::Obj(id));
        }
    }
    // 5. Top-K graveyards (own newest-first, then opponent newest-first; public zone, zero leak)
    for &id in game.player(seat).graveyard.iter().rev().take(GY_TOP_K) {
        if entries.len() >= MAX_ENT {
            break;
        }
        entries.push(EntEntry::Obj(id));
    }
    for &id in game.player(opp).graveyard.iter().rev().take(GY_TOP_K) {
        if entries.len() >= MAX_ENT {
            break;
        }
        entries.push(EntEntry::Obj(id));
    }

    let n_ent = entries.len();
    let mut o = Obs {
        glob: global_with_flags(game, seat, q, &flags),
        ents: Vec::with_capacity(n_ent * ENT_F),
        ent_ids: Vec::with_capacity(n_ent),
        ent_rel_idx: vec![NO_ENT; n_ent],
        ent_rel_type: vec![0; n_ent],
        acts: Vec::with_capacity(candidates.len() * ACT_F),
        act_ids: Vec::with_capacity(candidates.len()),
        act_ent_idx: vec![NO_ENT; candidates.len()],
    };

    // Fast ObjId -> entity token index lookup (separate maps for stack spells vs non-stack objects)
    let mut stack_tok: Vec<(ObjId, u8)> = Vec::new();
    let mut obj_tok: Vec<(ObjId, u8)> = Vec::with_capacity(n_ent);
    for (idx, entry) in entries.iter().enumerate() {
        match *entry {
            EntEntry::Stack(si) => {
                let s = &game.stack[si];
                let (feat, cid) = stack_entity(game, seat, s);
                o.ents.extend(feat);
                o.ent_ids.push(cid);
                if s.ability.is_none() {
                    stack_tok.push((s.card, idx as u8));
                }
            }
            EntEntry::Obj(id) => {
                o.ents.extend(entity_with_flags(game, seat, id, playable, false, &flags));
                o.ent_ids.push(card_id(&game.obj(id).def.name));
                obj_tok.push((id, idx as u8));
            }
        }
    }
    let find_tok = |oid: ObjId| -> Option<u8> {
        if (oid.0 as usize) < game.objs.len() && game.obj(oid).zone == Zone::Stack {
            stack_tok.iter().find(|(k, _)| *k == oid).map(|(_, v)| *v)
        } else {
            obj_tok.iter().find(|(k, _)| *k == oid).map(|(_, v)| *v)
        }
    };

    // Populate primary sparse relational edge per entity:
    //   1: stack_item -> targeted entity (spell on stack or permanent on battlefield)
    //   2: blocker -> attacker
    //   3: aura/equipment -> attached permanent
    //   4: stack ability -> source permanent on battlefield (fallback when no object target)
    for (idx, entry) in entries.iter().enumerate() {
        match *entry {
            EntEntry::Stack(si) => {
                let s = &game.stack[si];
                let mut linked = false;
                for t in s.targets.iter().flatten() {
                    if let Target::Obj(tid) = *t {
                        if let Some(ti) = find_tok(tid) {
                            if ti as usize != idx {
                                o.ent_rel_idx[idx] = ti;
                                o.ent_rel_type[idx] = 1;
                                linked = true;
                                break;
                            }
                        }
                    }
                }
                if !linked {
                    if let Some(src_i) = obj_tok.iter().find(|(k, _)| *k == s.card).map(|(_, v)| *v) {
                        if src_i as usize != idx {
                            o.ent_rel_idx[idx] = src_i;
                            o.ent_rel_type[idx] = 4;
                        }
                    }
                }
            }
            EntEntry::Obj(id) => {
                let obj = game.obj(id);
                if obj.zone == Zone::Battlefield {
                    if let Some(&atk) = obj.blocking.first() {
                        if let Some(ai) = obj_tok.iter().find(|(k, _)| *k == atk).map(|(_, v)| *v) {
                            o.ent_rel_idx[idx] = ai;
                            o.ent_rel_type[idx] = 2;
                        }
                    } else if let Some((att, _)) = obj.attached_to {
                        if let Some(ai) = obj_tok.iter().find(|(k, _)| *k == att).map(|(_, v)| *v) {
                            o.ent_rel_idx[idx] = ai;
                            o.ent_rel_type[idx] = 3;
                        }
                    }
                }
            }
        }
    }

    // Candidate actions + direct contextual entity pointer (`act_ent_idx`).
    // Reuse already-encoded entity features from `o.ents` when the action names a non-stack visible entity.
    for (i, a) in candidates.iter().enumerate() {
        if let Some(oid) = named(a) {
            if let Some(ei) = find_tok(oid) {
                o.act_ent_idx[i] = ei;
            }
            if (oid.0 as usize) < game.objs.len() {
                let hidden = game.obj(oid).zone == Zone::Hand && game.obj(oid).owner != seat;
                if !hidden {
                    if let Some(ei) = obj_tok.iter().find(|(k, _)| *k == oid).map(|(_, v)| *v as usize) {
                        let mut f = action_header(seat, q, a, i == 0);
                        f[16..].copy_from_slice(&o.ents[ei * ENT_F..(ei + 1) * ENT_F]);
                        o.acts.extend(f);
                        o.act_ids.push(o.ent_ids[ei]);
                        continue;
                    }
                }
            }
        }
        let (f, id) = action_with_flags(game, seat, q, a, i == 0, playable, &flags);
        o.acts.extend(f);
        o.act_ids.push(id);
    }
    o
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::duel::{new_duel, Deck};
    use crate::search::{candidates, Table};
    use playmat_engine::decide::Decider;
    use playmat_engine::rules;
    use playmat_engine::state::{Boost, Grant};

    fn ref_entity(game: &Game, seat: Seat, id: ObjId, playable: &[ObjId], hidden: bool) -> [f32; ENT_F] {
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
        let mut mark_color = |c: Color| {
            let i = match c {
                Color::W => 0,
                Color::U => 1,
                Color::B => 2,
                Color::R => 3,
                Color::G => 4,
            };
            f[28 + i] = 1.0;
        };
        if on_field {
            for c in game.colors(id) {
                mark_color(c);
            }
        } else {
            for &c in &def.colors {
                mark_color(c);
            }
        }
        for (i, k) in KEYWORDS.iter().enumerate() {
            f[33 + i] = if on_field { game.has_ability(id, k) } else { def.abilities.contains(k) } as u8 as f32;
        }
        f[46] = def.abilities.contains(&AbilityDef::FlashAbility) as u8 as f32;
        f[47] = playable.contains(&id) as u8 as f32;
        f[48] = def.subtypes.iter().any(|s| s == "Aura" || s == "Equipment") as u8 as f32;
        f
    }

    fn ref_action(
        game: &Game,
        seat: Seat,
        q: &Query,
        a: &Answer,
        quick: bool,
        playable: &[ObjId],
    ) -> ([f32; ACT_F], u32) {
        let mut f = action_header(seat, q, a, quick);
        let mut id = 0;
        if let Some(o) = named(a) {
            if (o.0 as usize) < game.objs.len() {
                let hidden = game.obj(o).zone == Zone::Hand && game.obj(o).owner != seat;
                f[16..].copy_from_slice(&ref_entity(game, seat, o, playable, false));
                if !hidden {
                    id = card_id(&game.obj(o).def.name);
                } else {
                    f[16..].fill(0.0);
                }
            }
        }
        (f, id)
    }

    fn ref_global(game: &Game, seat: Seat, q: &Query) -> Vec<f32> {
        let mut g: Vec<f32> = fitted::features(game, seat).iter().map(|v| *v as f32).collect();
        let opp = game.opponent(seat);
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
        let poison = |s: Seat| {
            game.player(s).counters.iter().filter(|(n, _)| n == "poison").map(|(_, v)| *v).sum::<i32>()
        };
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
        let ref_mana = |s: Seat| {
            let mut out = [0f32; 6];
            for &id in game.battlefield.iter() {
                let o = game.obj(id);
                if o.controller != s || o.tapped {
                    continue;
                }
                if game.has_type(id, CardType::Creature) && o.sick && !game.has_ability(id, &AbilityDef::HasteAbility)
                {
                    continue;
                }
                let mut can = [false; 6];
                for ab in &o.def.abilities {
                    if let Some(m) = ab.mana() {
                        for ci in 0..5 {
                            if m.colored[ci] > 0 {
                                can[ci] = true;
                            }
                        }
                        if m.colorless > 0 {
                            can[5] = true;
                        }
                    }
                }
                for ci in 0..6 {
                    if can[ci] {
                        out[ci] += 1.0 / 3.0;
                    }
                }
            }
            out
        };
        let ref_clock = |atk: Seat, def_s: Seat| {
            let mut p = 0i32;
            for &id in game.battlefield.iter() {
                let o = game.obj(id);
                if o.controller == atk
                    && o.attacking.is_some()
                    && !o.blocked
                    && game.has_type(id, CardType::Creature)
                {
                    p += game.power_toughness(id).0.max(0);
                }
            }
            let life = game.player(def_s).life.max(1) as f32;
            (p as f32 / life).min(2.0)
        };
        g.extend(ref_mana(seat));
        g.extend(ref_mana(opp));
        g.push(ref_clock(opp, seat));
        g.push(ref_clock(seat, opp));
        let top_targets_me = game.stack.last().is_some_and(|s| {
            s.targets.iter().flatten().any(|t| match *t {
                Target::Player(p) => p == seat,
                Target::Obj(oid) => (oid.0 as usize) < game.objs.len() && game.obj(oid).controller == seat,
            })
        });
        let top_is_spell = game.stack.last().is_some_and(|s| s.ability.is_none());
        g.push(top_targets_me as u8 as f32);
        g.push(top_is_spell as u8 as f32);
        g.extend(opponent_archetype_features(game, seat));
        let opp_untapped = lands(opp, true);
        let opp_total = lands(opp, false);
        let opp_draw_go = (game.active != Some(opp)
            && opp_untapped >= 2.0
            && them.spells_cast_last_turn == 0
            && !them.hand.is_empty()) as u8 as f32;
        let opp_open_frac = if opp_total > 0.0 { opp_untapped / opp_total } else { 0.0 };
        let tempo_life = (((them.life_lost + them.life_lost_last_turn) - (me.life_lost + me.life_lost_last_turn))
            as f32
            / 10.0)
            .clamp(-2.0, 2.0);
        g.extend([
            (me.spells_cast as f32 / 3.0).min(2.0),
            (them.spells_cast as f32 / 3.0).min(2.0),
            (them.spells_cast_last_turn as f32 / 3.0).min(2.0),
            (them.expended as f32 / 6.0).min(2.0),
            (me.expended as f32 / 6.0).min(2.0),
            opp_draw_go,
            opp_open_frac,
            tempo_life,
        ]);
        g
    }

    fn assert_obs_bit_exact(fast: &Obs, slow: &Obs, ctx: &str) {
        let bits = |xs: &[f32]| -> Vec<u32> { xs.iter().map(|v| v.to_bits()).collect() };
        assert_eq!(bits(&fast.glob), bits(&slow.glob), "{ctx}: glob mismatch");
        assert_eq!(bits(&fast.ents), bits(&slow.ents), "{ctx}: ents mismatch");
        assert_eq!(fast.ent_ids, slow.ent_ids, "{ctx}: ent_ids mismatch");
        assert_eq!(fast.ent_rel_idx, slow.ent_rel_idx, "{ctx}: ent_rel_idx mismatch");
        assert_eq!(fast.ent_rel_type, slow.ent_rel_type, "{ctx}: ent_rel_type mismatch");
        assert_eq!(bits(&fast.acts), bits(&slow.acts), "{ctx}: acts mismatch");
        assert_eq!(fast.act_ids, slow.act_ids, "{ctx}: act_ids mismatch");
        assert_eq!(fast.act_ent_idx, slow.act_ent_idx, "{ctx}: act_ent_idx mismatch");
        assert_eq!(fast.hash128(), slow.hash128(), "{ctx}: hash128 mismatch");
    }

    fn ref_observe(game: &Game, seat: Seat, q: &Query, cands: &[Answer]) -> Obs {
        let mut o = observe(game, seat, q, cands);
        o.glob = ref_global(game, seat, q);
        let playable: &[ObjId] = match q {
            Query::Priority { playable } => playable,
            _ => &[],
        };
        let opp = game.opponent(seat);
        let mut entries: Vec<EntEntry> = Vec::new();
        for si in (0..game.stack.len()).rev() {
            if entries.len() < MAX_ENT {
                entries.push(EntEntry::Stack(si));
            }
        }
        for &id in game.battlefield.iter().filter(|id| game.obj(**id).controller == seat) {
            if entries.len() < MAX_ENT {
                entries.push(EntEntry::Obj(id));
            }
        }
        for &id in game.battlefield.iter().filter(|id| game.obj(**id).controller != seat) {
            if entries.len() < MAX_ENT {
                entries.push(EntEntry::Obj(id));
            }
        }
        for &id in &game.player(seat).hand {
            if entries.len() < MAX_ENT {
                entries.push(EntEntry::Obj(id));
            }
        }
        for &id in game.exile.iter() {
            if entries.len() < MAX_ENT && !game.obj(id).exiled_face_down && game.may_play_now(id, seat) {
                entries.push(EntEntry::Obj(id));
            }
        }
        for &id in game.player(seat).graveyard.iter().rev().take(GY_TOP_K) {
            if entries.len() < MAX_ENT {
                entries.push(EntEntry::Obj(id));
            }
        }
        for &id in game.player(opp).graveyard.iter().rev().take(GY_TOP_K) {
            if entries.len() < MAX_ENT {
                entries.push(EntEntry::Obj(id));
            }
        }
        o.ents.clear();
        o.ent_ids.clear();
        for e in &entries {
            match *e {
                EntEntry::Stack(si) => {
                    let (feat, cid) = stack_entity(game, seat, &game.stack[si]);
                    o.ents.extend(feat);
                    o.ent_ids.push(cid);
                }
                EntEntry::Obj(id) => {
                    o.ents.extend(ref_entity(game, seat, id, playable, false));
                    o.ent_ids.push(card_id(&game.obj(id).def.name));
                }
            }
        }
        o.acts.clear();
        o.act_ids.clear();
        for (i, a) in cands.iter().enumerate() {
            let (f, id) = ref_action(game, seat, q, a, i == 0, playable);
            o.acts.extend(f);
            o.act_ids.push(id);
        }
        o
    }

    #[test]
    fn observe_fast_path_is_bit_exact_across_duel_and_layer_effects() {
        let db = playmat_engine::cards();
        let d0 = Deck::parse(
            "red_white",
            "12 Mountain\n12 Plains\n8 Lightning Strike\n8 Shock\n8 Grizzly Bears\n6 Serra Angel\n6 Glorious Anthem\n",
        );
        let d1 = Deck::parse(
            "blue_green",
            "12 Island\n12 Forest\n8 Cancel\n8 Unsummon\n8 Llanowar Elves\n6 Air Elemental\n6 Giant Growth\n",
        );
        let mut game = new_duel(&db, [&d0, &d1], 42, 0).expect("new_duel");
        struct ParityDecider {
            inner: Table,
            checked: usize,
        }
        impl Decider for ParityDecider {
            fn answer(&mut self, game: &Game, seat: Seat, q: &Query) -> Answer {
                let ans = self.inner.answer(game, seat, q);
                let cands = candidates(game, seat, q, &ans, 24);
                for s in [0usize, 1usize] {
                    let fast = observe(game, s, q, &cands);
                    let slow = ref_observe(game, s, q, &cands);
                    assert_obs_bit_exact(&fast, &slow, &format!("turn {} step {:?} seat {s}", game.turn, game.step));
                }
                self.checked += 1;
                ans
            }
            fn stopped(&self) -> bool {
                self.inner.stopped()
            }
        }
        let mut pd = ParityDecider { inner: Table::new(vec![None, None]), checked: 0 };
        rules::play_from_priority(&mut game, &mut pd, 10);
        assert!(pd.checked >= 20, "expected >= 20 decisions checked, got {}", pd.checked);

        // Also test continuous-effect edge cases: +1/+1 counters, keyword counters, switched boosts, grants,
        // lost_until_eot, color_overrides, and stack-spell targeting!
        let mut g2 = new_duel(&db, [&d0, &d1], 99, 0).expect("new_duel 2");
        let hand_c0: Vec<ObjId> = g2
            .player(0)
            .hand
            .iter()
            .copied()
            .filter(|id| g2.obj(*id).def.types.contains(&CardType::Creature))
            .collect();
        if let Some(&cid) = hand_c0.first() {
            g2.player_mut(0).hand.retain(|x| *x != cid);
            g2.obj_mut(cid).zone = Zone::Battlefield;
            std::sync::Arc::make_mut(&mut g2.battlefield).push(cid);
            g2.obj_mut(cid).add_counters("+1/+1", 2);
            g2.obj_mut(cid).add_counters("flying", 1);
            let zcc = g2.obj(cid).zcc;
            std::sync::Arc::make_mut(&mut g2.boosts).push(Boost {
                target: cid,
                zcc,
                power: 3,
                toughness: -1,
                switch: true,
                noncreature_too: false,
                until_end_of_turn: true,
                until_turn_of: None,
            });
            std::sync::Arc::make_mut(&mut g2.grants).push(Grant {
                target: cid,
                zcc,
                ability: AbilityDef::TrampleAbility,
                until_end_of_turn: true,
                until_turn_of: None,
            });
            g2.color_overrides.push((cid, zcc, vec![Color::B, Color::G], false, true));
            let q = Query::Target { players: vec![0, 1], objs: vec![cid], required: true };
            let cands = vec![Answer::Obj(cid), Answer::Player(1)];
            let fast = observe(&g2, 0, &q, &cands);
            let slow = ref_observe(&g2, 0, &q, &cands);
            assert_obs_bit_exact(&fast, &slow, "synthetic boosts+grants+color_overrides");

            g2.lost_until_eot.push((cid, zcc, AbilityDef::FlyingAbility, g2.turn));
            let fast2 = observe(&g2, 0, &q, &cands);
            let slow2 = ref_observe(&g2, 0, &q, &cands);
            assert_obs_bit_exact(&fast2, &slow2, "synthetic lost_until_eot");
        }
    }
}
