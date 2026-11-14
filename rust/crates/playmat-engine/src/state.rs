//! The game state. Each part sits behind an `Arc` and is copied only when changed (`Arc::make_mut`), so copying a
//! `Game` for an AI search node bumps a few reference counts.

use crate::card::{
    AbilityDef, Amount, CardDef, CardType, Condition, Duration, EffectDef, Filter, Mana, Pred, Trigger, Value,
};
use crate::rng::JavaRandom;
use std::sync::Arc;

pub type Seat = usize;

/// An object's number, given in the order objects are created.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ObjId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Zone {
    Library,
    Hand,
    Battlefield,
    Graveyard,
    Stack,
    Exile,
    /// emblems
    Command,
    /// outside the game: a sideboard's cards (Learn's Lessons)
    Outside,
}

/// The steps of a turn, named as Java's `PhaseStep`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Step {
    Untap,
    Upkeep,
    Draw,
    PrecombatMain,
    BeginCombat,
    DeclareAttackers,
    DeclareBlockers,
    FirstCombatDamage,
    CombatDamage,
    EndCombat,
    PostcombatMain,
    EndTurn,
    Cleanup,
}

impl Step {
    pub fn name(self) -> &'static str {
        match self {
            Step::Untap => "UNTAP",
            Step::Upkeep => "UPKEEP",
            Step::Draw => "DRAW",
            Step::PrecombatMain => "PRECOMBAT_MAIN",
            Step::BeginCombat => "BEGIN_COMBAT",
            Step::DeclareAttackers => "DECLARE_ATTACKERS",
            Step::DeclareBlockers => "DECLARE_BLOCKERS",
            Step::FirstCombatDamage => "FIRST_COMBAT_DAMAGE",
            Step::CombatDamage => "COMBAT_DAMAGE",
            Step::EndCombat => "END_COMBAT",
            Step::PostcombatMain => "POSTCOMBAT_MAIN",
            Step::EndTurn => "END_TURN",
            Step::Cleanup => "CLEANUP",
        }
    }

    pub fn parse(name: &str) -> Option<Step> {
        Some(match name {
            "UNTAP" => Step::Untap,
            "UPKEEP" => Step::Upkeep,
            "DRAW" => Step::Draw,
            "PRECOMBAT_MAIN" => Step::PrecombatMain,
            "BEGIN_COMBAT" => Step::BeginCombat,
            "DECLARE_ATTACKERS" => Step::DeclareAttackers,
            "DECLARE_BLOCKERS" => Step::DeclareBlockers,
            "FIRST_COMBAT_DAMAGE" => Step::FirstCombatDamage,
            "COMBAT_DAMAGE" => Step::CombatDamage,
            "END_COMBAT" => Step::EndCombat,
            "POSTCOMBAT_MAIN" => Step::PostcombatMain,
            "END_TURN" => Step::EndTurn,
            "CLEANUP" => Step::Cleanup,
            _ => return None,
        })
    }

    pub fn is_main(self) -> bool {
        matches!(self, Step::PrecombatMain | Step::PostcombatMain)
    }
}

/// How long a control change (`GainControlTargetEffect`, layer 2) lasts
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlUntil {
    /// for the rest of the game
    Forever,
    /// until the cleanup step
    EndOfTurn,
    /// while the new controller controls the source (`Duration.WhileControlled`)
    WhileControlled(ObjId, u32),
    /// until the source leaves the battlefield (`Duration.UntilSourceLeavesBattlefield`)
    SourceLeaves(ObjId, u32),
}

/// A permanent's control change: who controls it, until when, and who controlled it before
#[derive(Clone, Debug)]
pub struct ControlChange {
    pub target: ObjId,
    /// the permanent's zone change count: the change ends when it leaves the battlefield
    pub zcc: u32,
    pub to: Seat,
    pub before: Seat,
    pub until: ControlUntil,
}

/// A card in some zone, or a permanent.
#[derive(Clone, Debug)]
pub struct Obj {
    pub id: ObjId,
    /// zone changes so far: an object that changes zones is a new object (rule 400.7), which targets and effects
    /// pointing at the old one don't follow
    pub zcc: u32,
    pub def: Arc<CardDef>,
    pub owner: Seat,
    pub controller: Seat,
    pub zone: Zone,
    pub tapped: bool,
    pub damage: i32,
    pub counters: Vec<(String, i32)>,
    /// its counters as it last left the battlefield
    pub lki_counters: Vec<(String, i32)>,
    /// deathtouch and lifelink it had as it last left the battlefield, granted ones too (Vial Smasher's lifelink
    /// from Eternal Thirst, put into the graveyard by the legend rule)
    pub lki_keywords: Vec<AbilityDef>,
    /// what it was attached to as it last left the battlefield (Path to Redemption's "Sacrifice this: Exile enchanted
    /// creature")
    pub lki_attached_to: Option<(ObjId, u32)>,
    /// the permanents attached to it as it last left the battlefield, with their zone-change counts (Zack Fair's
    /// "attach an Equipment that was attached to {this}")
    pub lki_attachments: Vec<(ObjId, u32)>,
    /// its abilities as it last left the battlefield, when effects gave it some (land_type_base): what it triggers
    /// on from the others leaving with it (Eternal Thirst's "Whenever a creature an opponent controls dies" on a
    /// creature dying in the same combat)
    pub lki_def: Option<Arc<CardDef>>,
    pub face_down: bool,
    /// how it came to be face down: the name and image its views show (`BecomesFaceDownCreatureEffect.FaceDownType`)
    pub face_down_as: Option<crate::card::FaceDown>,
    /// face down (cast with morph or disguise, manifested, cloaked) or prototyped: the card it is, while `def` is the
    /// 2/2 face-down creature (`CardDef::face_down`) or the prototype; its card again when it's turned face up or
    /// leaves the stack or the battlefield
    pub face_up: Option<Arc<CardDef>>,
    /// the player it attacks
    pub attacking: Option<Seat>,
    /// the planeswalker it attacks (and its zone change count), if it attacks one
    pub attacking_pw: Option<(ObjId, u32)>,
    /// a battle's protector
    pub protector: Option<Seat>,
    /// a loyalty ability of it was activated this turn
    pub loyalty_used: bool,
    /// it entered the battlefield as a kicked spell
    pub kicked: bool,
    /// it entered the battlefield by resolving as a spell
    pub cast: bool,
    /// `cast` as it last left the battlefield (an ability of it that resolves later looks back)
    pub cast_lki: bool,
    /// its toughness as it last left the battlefield (Righteous Valkyrie's trigger on a creature already gone)
    pub toughness_lki: Option<i32>,
    /// its power as it last left the battlefield (`SourcePermanentPowerValue`'s last known information)
    pub power_lki: Option<i32>,
    /// +1/+1 counters were put on it since triggers were last checked (`COUNTERS_ADDED`)
    pub p1p1_added: bool,
    /// it has flashback for its mana cost this turn (Stingcaster Mage), while it stays in the graveyard
    pub flashback_turn: Option<u32>,
    /// a Case that was solved
    pub solved: bool,
    /// what it was before an effect changed its characteristics (`def`) until end of turn
    pub until_end_of_turn_def: Option<Arc<CardDef>>,
    /// a static "as long as …, it's a creature" applied: the card as it is without it
    pub becomes_base: Option<Arc<CardDef>>,
    /// the Auras whose "enchanted … is a …" (`BecomesCreatureAttachedEffect`) `def` has, in timestamp order
    pub becomes_auras: Vec<ObjId>,
    /// lands given basic land types by a static (Urborg): the land without them, and the types given
    pub land_type_base: Option<(Arc<CardDef>, Vec<String>, Vec<AbilityDef>)>,
    /// the creature type chosen as it entered (Cavern of Souls)
    pub chosen_type: Option<String>,
    /// the card name chosen as it entered (Petrified Hamlet), while it's on the battlefield
    pub chosen_name: Option<String>,
    /// the lines a permanent adds to its rules in a view, by key (`Permanent.addInfo`: "Chosen color: Black")
    pub info: Vec<(String, String)>,
    /// prepared: the copy of its prepare spell in exile
    pub prepared_copy: Option<ObjId>,
    /// a prepared copy: the permanent (and its zone change count) whose copy it is
    pub prepared_for: Option<(ObjId, u32)>,
    /// in exile, in the zone of the source (and its zone change count) that exiled it (`CardUtil.getExileZoneId`)
    pub exiled_with: Option<(ObjId, u32)>,
    /// it transformed on the battlefield (not entered transformed): its card's other face is a card of its own
    pub transformed_in_place: bool,
    /// a Room permanent's doors, left and right: unlocked or not
    pub doors: [bool; 2],
    /// the half of a Room cast (0 left, 1 right): its door unlocks as the Room enters
    pub room_cast: Option<usize>,
    /// a fused split card being cast with both halves: its mana is paid before targets (`Spell.activate`)
    pub fused_cast: bool,
    /// the turn it was foretold (exiled face down from hand), while it stays in exile
    pub foretold_turn: Option<u32>,
    /// the turn it became plotted, while it stays in exile
    pub plotted_turn: Option<u32>,
    /// suspended: in exile with time counters, from its suspend ability
    pub suspended: bool,
    /// the zone it was cast from, as a spell
    pub cast_from: Option<Zone>,
    /// a spell Cavern of Souls' mana was spent on: it can't be countered
    pub uncounterable: bool,
    /// the X it was cast with, while it's on the battlefield
    pub cast_x: i32,
    /// what it was before an effect changed its characteristics (`def`) for as long as it stays on the battlefield
    pub printed_def: Option<Arc<CardDef>>,
    /// a land's card as it was before Leyline of the Guildpact made it every basic land type
    pub pre_guildpact: Option<Arc<CardDef>>,
    /// under it, each ability's index in `pre_guildpact` (None: the any-color one it added)
    pub guildpact_order: Vec<Option<usize>>,
    /// what it entered as a copy of (Mockingbird): its copiable values, which a copy of it copies
    pub copied_def: Option<Arc<CardDef>>,
    /// what it became a copy of until end of turn (Mirage Mirror): the latest copy effect, so its copiable values
    pub eot_copy: Option<Arc<CardDef>>,
    /// the permanent (and its zone change count) this Aura is attached to
    pub attached_to: Option<(ObjId, u32)>,
    /// cast as its adventure: the whole card, while `def` is the adventure's
    pub adventurer: Option<Arc<CardDef>>,
    /// a modal double-faced card played as its back face: the whole card, back when it leaves the battlefield
    pub front: Option<Arc<CardDef>>,
    /// a spell cast with flashback: exiled instead of going anywhere else from the stack
    pub flashback: bool,
    /// harmonize: the creature chosen to tap for the cost, while this is cast
    pub harmonize_tap: Option<ObjId>,
    /// cast with its escape cost: on the stack, and on the battlefield as the permanent it became (cleared as it leaves)
    pub escaped: bool,
    /// the player the attacker this ninja's ninjutsu returned was attacking
    pub ninjutsu_defender: Option<Seat>,
    /// and the planeswalker it attacked, if one
    pub ninjutsu_pw: Option<(ObjId, u32)>,
    /// the permanent chosen as it entered (`ChooseCreatureEffect`), kept after it leaves
    pub chosen: Option<(ObjId, u32)>,
    /// a Class's level (1 until it gains another)
    pub class_level: u32,
    /// the level it had as it last left the battlefield (its look-back: Scavenger's Talent sacrificed with others)
    pub lki_class_level: u32,
    /// exerted: doesn't untap during its controller's next untap step
    pub exerted: bool,
    /// its power-up ability was activated (once each game, for this object)
    pub powered_up: bool,
    /// keyword statuses a permanent has while it stays (renowned, an exhaust ability activated), by name
    pub marks: Vec<String>,
    /// creature types and base power and toughness it has until it leaves the battlefield (Evolved Sleeper: each
    /// "becomes" replaces its creature types)
    pub creature_types: Option<Vec<String>>,
    pub base_pt: Option<(i32, i32)>,
    /// when `base_pt` was set (`Game::timestamp`): layer 7b applies in timestamp order
    pub base_pt_at: u64,
    /// when it last entered the battlefield (`Game::timestamp`): its static abilities' effects' timestamp
    pub entered_at: u64,
    /// regeneration shields until end of turn
    pub regen_shields: u32,
    /// "if this creature would die, exile it instead" for as long as it stays (Can't Stay Away)
    pub exile_instead_of_dying: bool,
    /// unearthed: exiled if it would leave the battlefield (`UnearthLeavesBattlefieldEffect`)
    pub unearthed: bool,
    /// `DamagedByControlledWatcher`: the turn, and the players whose sources dealt it damage that turn (bits)
    pub damaged_by: (u32, u8),
    /// it lost all abilities while this source (and zone-change count) is on the battlefield: the source, and its
    /// card as it was
    pub lost_abilities: Option<(ObjId, u32, Arc<CardDef>)>,
    /// in exile: this player may play it through the end of this turn number
    pub may_play: Option<(Seat, u32)>,
    /// `may_play` ends at the beginning of that turn's end step instead (`Duration.UntilYourNextEndStep`)
    pub may_play_end_step: bool,
    /// `may_play` lasts only while its player controls this permanent, as it was (`Duration.WhileControlled`)
    pub may_play_while: Option<(ObjId, u32)>,
    /// mana of any type can be spent to cast it (while it may be played from exile)
    pub any_mana: bool,
    /// while it may be played from exile, it's cast without paying its mana cost (Oracle's Vault)
    pub play_free: bool,
    /// it may be played from exile only by casting it (not played as a land)
    pub cast_only: bool,
    /// cast from exile, it costs this much more (Elite Spellbinder)
    pub play_tax: u32,
    /// cast for its warp cost (on the stack, then on the battlefield)
    pub warped: bool,
    /// cast for its evoke cost
    pub evoked: bool,
    /// cast for its impending cost (kept from the stack to the battlefield)
    pub impending: bool,
    /// cast for its dash cost
    pub dashed: bool,
    /// the mana of each color spent to cast it (WUBRG)
    pub spent_colors: [u32; 5],
    /// cast for its blitz cost (until it leaves the battlefield)
    pub blitzed: bool,
    /// cast with its offspring cost paid
    pub offspring: bool,
    /// in exile after warp: its owner may cast it from this turn number on
    pub warp_from: Option<u32>,
    /// the mana spent to cast it, as a spell (`ManaSpentToCastCount`)
    pub mana_spent: u32,
    /// the turn it last entered the battlefield
    pub entered_turn: u32,
    /// the turn and how many times this permanent became a target in it
    pub targeted: (u32, u32),
    /// exiled "on an adventure": its owner may cast the creature from exile
    pub on_adventure: bool,
    /// the zone Java keeps for an adventurer card's adventure part (`CardWithSpellOption`): it follows the card,
    /// except when the card goes onto the stack or the battlefield or leaves the battlefield
    pub adventure_zone: Zone,
    /// when it was attached (`Game.attach_clock`)
    pub attached_at: u64,
    /// where what it gives as an attachment ("equipped creature has …") registers in Java's trigger order: after
    /// every object there was as it was attached, as a Class level's ability does
    pub attached_key: u64,
    /// the attackers it blocks
    pub blocking: Vec<ObjId>,
    /// how many attackers it was declared to block (Java's `blocking` count: an attacker leaving combat doesn't
    /// lower it)
    pub blocks: u32,
    /// blocked this combat (stays blocked if its blockers leave)
    pub blocked: bool,
    /// an attacker's blockers, in the order they were declared (Java's `CombatGroup.blockers`)
    pub blocked_by: Vec<ObjId>,
    /// came under its controller's control since their most recent turn began (summoning sickness)
    pub sick: bool,
    /// a token made from a token class: the class, the set its image was looked for in, and which image the random
    /// pick took (`TokenRepository.findPreferredTokenInfo`), for its view
    pub token_image: Option<(String, String, u32)>,
    /// dealt damage by a source with deathtouch since the last check
    pub deathtouched: bool,
    /// a token: it exists only on the battlefield
    pub token: bool,
    /// where its triggered abilities sit in Java's `TriggeredAbilities` map (registered as cards are loaded, tokens as
    /// they're created): abilities that trigger on the same event wait in this order
    pub trigger_order: u64,
    /// a Class's levels gained (on the battlefield), with where the abilities they add are in trigger order
    pub class_gained: Vec<(u32, u64)>,
}

impl Obj {
    /// Where Java's numbering of its abilities starts: a permanent showing its back face (entered transformed, a
    /// battle's back face cast, or transformed on the battlefield) has its back face's card's abilities, whose spell
    /// ability comes first (`TransformingDoubleFacedCardHalf`, `PermanentCard.copyFromCard`: Brigid, Doun's Mind's
    /// {G} is `a1`)
    pub fn java_ability_offset(&self) -> usize {
        usize::from(self.zone == Zone::Battlefield && self.front.as_ref().is_some_and(|f| f.transform_back.is_some()))
    }

    /// Sets a line of its view's rules (`Permanent.addInfo`, with `CardUtil.addToolTipMarkTags`)
    pub fn add_info(&mut self, key: &str, text: &str) {
        let line = format!("<font color = 'blue'>{}</font>", text);
        match self.info.iter_mut().find(|(k, _)| k == key) {
            Some(x) => x.1 = line,
            None => self.info.push((key.to_string(), line)),
        }
    }

    /// Its copiable values: what it's a copy of, or its card before other effects changed it
    pub fn copiable(&self) -> Arc<CardDef> {
        // (an Aura's or a static's "is a …" isn't copiable: Frogify's creature is copied as it's printed)
        self.eot_copy
            .clone()
            .or_else(|| self.copied_def.clone())
            .or_else(|| self.printed_def.clone())
            .or_else(|| self.until_end_of_turn_def.clone().filter(|_| self.becomes_base.is_none()))
            .or_else(|| self.becomes_base.clone())
            .or_else(|| self.until_end_of_turn_def.clone())
            .unwrap_or_else(|| self.def.clone())
    }

    pub fn name(&self) -> &str {
        &self.def.name
    }

    pub fn is(&self, t: CardType) -> bool {
        self.def.is(t)
    }

    pub fn has(&self, ability: &AbilityDef) -> bool {
        self.def.has(ability)
    }

    pub fn counter(&self, name: &str) -> i32 {
        self.counters.iter().filter(|(n, _)| n == name).map(|(_, k)| *k).sum()
    }

    pub fn add_counters(&mut self, name: &str, amount: i32) {
        if amount > 0 && name == "+1/+1" {
            self.p1p1_added = true;
        }
        match self.counters.iter_mut().find(|(n, _)| n == name) {
            Some(c) => c.1 += amount,
            None => self.counters.push((name.to_string(), amount)),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Player {
    /// `UserData.autoTargetLevel` 2 (Java's AI players): auto-choice picks any target, feel-bad or not
    pub auto_target_all: bool,
    /// asked at every priority, in every step: a seat whose skip settings aren't the default ones (Java logs its passes)
    pub every_priority: bool,
    /// `UserData.confirmEmptyManaPool` off: passes with mana in the pool without being asked
    pub no_pool_check: bool,
    /// mana in the pool always pays automatically: the Java AI's pool, whose stock (`ManaPool.setStock`) is set on
    /// the copy each dialog's stand-in player gets, never on its own
    pub no_stock: bool,
    /// the cards in its sideboard (only counted: `OpenSideboardHint`)
    pub sideboard: usize,
    /// the sideboard's cards, when the game knows them (shadow games' `sideboardCards`), in Java's order: what
    /// Learn can reveal
    pub outside: Vec<ObjId>,
    pub life: i32,
    pub counters: Vec<(String, i32)>,
    pub hand: Vec<ObjId>,
    /// top first
    pub library: Vec<ObjId>,
    pub graveyard: Vec<ObjId>,
    pub lost: bool,
    pub won: bool,
    /// the game is a draw for them (`Player.drew`: an infinite loop they agreed to call a draw)
    pub drew: bool,
    pub lands_played: u32,
    pub mana_pool: Mana,
    /// the pool's mana one by one in the order it was made (kind 0-4 WUBRG, 5 colorless), and whether a snow
    /// permanent made it: Java's `ManaPoolItem`s, which a payment takes from the first that has the kind, and whose
    /// source a snow symbol's filter checks. Kept in line with `mana_pool` by `sync_pool_units`
    pub pool_units: Vec<(u8, bool)>,
    /// how much of the pool's {R} lasts until end of combat (firebending)
    pub combat_red: u32,
    /// how much of each kind of the pool's mana lasts until end of turn (`UntilEndOfTurnManaEffect`: mana added with
    /// Java's `emptyOnTurnsEnd`, kept as steps and phases end until the end phase)
    pub eot_mana: Mana,
    /// conditional mana made for a payment that couldn't spend it: (the mana, its source, the ability's index); a
    /// later spell it can pay for spends it (`ConditionalMana` in Java's pool)
    pub conditional_pool: Vec<(Mana, ObjId, usize)>,
    /// the pool as the player last acted at priority (`ManaPool.setStock`): mana there since isn't fresh, and
    /// a payment spends only fresh mana unless the player unlocks it
    pub mana_stock: Mana,
    pub passed: bool,
    /// drew from an empty library since the last state-based check
    pub drew_from_empty: bool,
    /// can't gain life for the rest of the game (Screaming Nemesis)
    pub cant_gain_life: bool,
    /// declared an attacker this turn (`PlayerAttackedWatcher`)
    pub attacked: bool,
    /// the attackers declared this turn, over every combat (`PlayerAttackedWatcher.getNumberOfAttackersCurrentTurn`)
    pub attackers_this_turn: u32,
    /// life lost this turn
    pub life_lost: i32,
    /// cards drawn this turn
    pub drawn: u32,
    /// of those, drawn during a draw step (`CardsDrawnDuringDrawStepWatcher`)
    pub drawn_in_draw_step: u32,
    /// noncreature spells cast this turn
    pub noncreature_cast: u32,
    /// cast a non-Lemur creature spell with flying this turn
    pub flier_cast: bool,
    /// times they gained life this turn
    pub life_gains: u32,
    /// life they gained this turn
    pub life_gained: i32,
    /// spells they cast this turn, and how many of them were instants or sorceries
    pub spells_cast: u32,
    pub instants_sorceries_cast: u32,
    /// spells they cast last turn (`CastSpellLastTurnWatcher.getAmountOfSpellsCastOnPrevTurn`)
    pub spells_cast_last_turn: u32,
    /// permanents sacrificed this turn (`PermanentsSacrificedWatcher`)
    pub sacrificed: u32,
    /// mana spent on spells this turn (`ExpendWatcher`)
    pub expended: u32,
    /// lands that entered under their control this turn (`LandfallWatcher`)
    pub lands_entered: u32,
    /// can't cast noncreature spells during this turn (an opponent's Ranger-Captain of Eos)
    pub no_noncreature_turn: Option<u32>,
    /// can't cast spells during this turn (an opponent's Silence)
    pub no_spells_turn: Option<u32>,
    /// a permanent they controlled left the battlefield this turn (`RevoltWatcher`)
    pub permanent_left: bool,
    /// the city's blessing (ascend: `DesignationType.CITYS_BLESSING`), for the rest of the game
    pub citys_blessing: bool,
    /// nonland permanents that entered under their control this turn (celebration)
    pub nonlands_entered: u32,
    /// permanent cards put into their graveyard this turn (`DescendedWatcher`)
    pub descended: u32,
    /// life lost last turn (`PlayerLostLifeWatcher.getLifeLostLastTurn`)
    pub life_lost_last_turn: i32,
}

impl Player {
    /// `pool_units` in line with `mana_pool`: mana added without a unit (an effect's) comes last and isn't snow;
    /// mana gone without one (undone, emptied) leaves from the last units of its kind
    pub fn sync_pool_units(&mut self) {
        for k in 0..6u8 {
            let have = if k < 5 { self.mana_pool.colored[k as usize] } else { self.mana_pool.colorless } as usize;
            let mut n = self.pool_units.iter().filter(|u| u.0 == k).count();
            while n > have {
                if let Some(at) = self.pool_units.iter().rposition(|u| u.0 == k) {
                    self.pool_units.remove(at);
                }
                n -= 1;
            }
            for _ in n..have {
                self.pool_units.push((k, false));
            }
        }
    }

    /// mana made by a source, snow or not, into the pool
    pub fn add_pool_mana(&mut self, m: Mana, snow: bool) {
        self.sync_pool_units();
        for k in 0..6u8 {
            let n = if k < 5 { m.colored[k as usize] } else { m.colorless };
            for _ in 0..n {
                self.pool_units.push((k, snow));
            }
        }
        for c in 0..5 {
            self.mana_pool.colored[c] += m.colored[c];
        }
        self.mana_pool.colorless += m.colorless;
    }
}

/// What a target is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Player(Seat),
    Obj(ObjId),
}

/// Cards revealed while an object resolves (`GameState.revealed`, cleared at priority): its source, the source's zone
/// change count, whether the title has it in parentheses (`DiscardCardYouChooseTargetEffect`) and the cards with the
/// zones they were revealed from
#[derive(Clone, Debug)]
pub struct Revealed {
    pub source: ObjId,
    pub zcc: u32,
    pub paren: bool,
    pub cards: Vec<(ObjId, Zone)>,
}

/// Cards a player looked at while an object resolves (`GameState.lookedAt`, `Player.lookAtCards`): who looked, the
/// source and its zone change count (in the title unless `bare`: Mishra's Bauble's is its id name alone), the cards
#[derive(Clone, Debug)]
pub struct LookedAt {
    pub viewer: Seat,
    pub source: ObjId,
    pub zcc: u32,
    pub bare: bool,
    pub cards: Vec<ObjId>,
}

/// A combat requirement a resolved effect made (`RequirementEffect` until end of turn or of combat)
#[derive(Clone, Debug, PartialEq)]
pub struct Requirement {
    pub kind: RequirementKind,
    /// the permanent it's about, with its zone change count
    pub obj: (ObjId, u32),
    /// `MustBlock`: the attacker it must block, with its zone change count
    pub attacker: Option<(ObjId, u32)>,
    pub end_of_combat: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequirementKind {
    /// all creatures able to block it do so
    BlockedByAll,
    /// it attacks if able
    Attacks,
    /// it blocks `attacker` if able
    MustBlock,
}

/// A combat group as views show it (`Combat.groups`): an attacker, in the order they attacked, and once it's left the
/// battlefield what its group was then (Java keeps the group: its defender, blocked, and blockers)
#[derive(Clone, Debug)]
pub struct CombatGroup {
    pub attacker: ObjId,
    pub left: Option<(Seat, Option<ObjId>, bool, Vec<ObjId>)>,
}

/// A card spliced onto a spell: the card, its targets and their zone change counters.
pub type Spliced = (ObjId, Vec<Vec<Target>>, Vec<Vec<u32>>);

/// The event the triggers being checked trigger on, as far as `AdditionalTriggersEffect` asks
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TriggerCause {
    /// a permanent entering
    Entered(ObjId),
    /// a permanent dying
    Died(ObjId),
}

#[derive(Clone, Debug)]
pub struct StackItem {
    /// the spell's card, or the source of an ability
    pub card: ObjId,
    /// an ability's face, when its source has transformed since it triggered or was activated
    pub def: Option<Arc<CardDef>>,
    pub controller: Seat,
    /// the times its replicate cost was paid
    pub replicated: u32,
    /// the chosen targets, per target of the spell or ability (a target can take several, "up to two")
    pub targets: Vec<Vec<Target>>,
    /// each object target's zone-change count when it was chosen (0 for players)
    pub target_zcc: Vec<Vec<u32>>,
    /// for an ability: its index in the source's abilities, and the source's zone-change count when it triggered
    pub ability: Option<(usize, u32)>,
    /// a spell cast with its kicker paid
    pub kicked: bool,
    /// the source's power as it last was on the battlefield, for an ability whose cost sacrificed it
    pub lki_power: Option<i32>,
    /// the source's toughness as it last was on the battlefield, for its own leave triggers
    /// (`SourcePermanentToughnessValue`'s last known information)
    pub lki_toughness: Option<i32>,
    /// the greatest toughness among the permanents its cost sacrificed (Witch's Oven)
    pub sacrificed_toughness: Option<i32>,
    /// the total power, toughness and mana value of the permanents its costs sacrificed (`SacrificeCostCreaturesPower`,
    /// `SacrificeCostCreaturesToughness`, `SacrificeCostManaValue`)
    pub sacrificed_stats: [i32; 3],
    /// a divided target's share for each of its first target group's targets, in order
    pub target_amounts: Vec<i32>,
    /// a delayed triggered ability (it has no ability index)
    pub delayed: Option<Arc<Delayed>>,
    /// a modal spell's chosen modes, in printed order (empty: the first)
    pub modes: Vec<usize>,
    /// the X announced as it was cast or activated
    pub x: i32,
    /// a ward trigger: the ability it counters (its source and its `ability`); a spell is its target instead
    pub ward_of: Option<(ObjId, (usize, u32))>,
    /// an evolve trigger: the creature that entered
    pub entering: Option<ObjId>,
    /// gift: promised as it was cast, and the opponent it was promised to
    pub gift_promised: bool,
    pub gift_to: Option<Seat>,
    /// cards spliced onto this spell: each card's spell ability resolves after the spell's own, with its targets
    /// and their zone-change counts
    pub spliced: Vec<Spliced>,
    /// a cast trigger: the spell that was cast (Java's `getValue("spellCast")`), whatever its target pointer is
    pub cast_spell: Option<ObjId>,
}

/// An ability a resolved effect gave one object.
#[derive(Clone, Debug, PartialEq)]
pub struct Grant {
    pub target: ObjId,
    pub zcc: u32,
    pub ability: AbilityDef,
    pub until_end_of_turn: bool,
    /// it lasts until this player's next turn
    pub until_turn_of: Option<Seat>,
}

/// A static handler's effect that started as a spell or ability resolved (`EffectDef::CardStatic`): it lasts until
/// the turn's cleanup
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedStatic {
    pub effect: EffectDef,
    pub source: ObjId,
    pub controller: Seat,
    pub timestamp: u64,
    /// what it points at (the resolving ability's targets, or its handler's choice), with their zone-change counts
    pub targets: Vec<(Target, u32)>,
    /// `Duration.UntilYourNextTurn` (its first argument): it lasts until this player's next turn begins, not the
    /// turn's cleanup
    pub until_turn_of: Option<Seat>,
}

/// A delayed triggered ability a resolved effect set up: "sacrifice that token at end of combat".
#[derive(Clone, Debug, PartialEq)]
pub struct Delayed {
    pub rule: String,
    /// the permanents it sacrifices, with their zone-change counts
    pub sacrifice: Vec<(ObjId, u32)>,
    /// warp: the permanent it exiles (castable by its owner from exile on a later turn)
    pub warp_exile: Vec<(ObjId, u32)>,
    /// a reflexive trigger's effects, from its source as it was (`source_zcc`)
    pub effects: Vec<crate::card::EffectDef>,
    pub source_zcc: u32,
    /// the permanents its effects act on, while they're still those objects (`FixedTarget`)
    pub fixed: Vec<(ObjId, u32)>,
    /// earthbend: the card it returns to the battlefield tapped, while it's still that card in a graveyard or exile
    pub return_tapped: Vec<(ObjId, u32)>,
    /// a reflexive trigger's targets, chosen as it goes on the stack
    pub targets: Vec<crate::card::TargetDef>,
    /// its intervening "if", checked as it triggers and as it resolves
    pub condition: Option<crate::card::Condition>,
}

impl StackItem {
    pub fn spell(card: ObjId, controller: Seat) -> StackItem {
        StackItem {
            card,
            controller,
            replicated: 0,
            targets: Vec::new(),
            target_zcc: Vec::new(),
            ability: None,
            kicked: false,
            lki_power: None,
            lki_toughness: None,
            sacrificed_toughness: None,
            sacrificed_stats: [0; 3],
            target_amounts: Vec::new(),
            delayed: None,
            modes: Vec::new(),
            x: 0,
            ward_of: None,
            entering: None,
            gift_promised: false,
            gift_to: None,
            def: None,
            spliced: Vec::new(),
            cast_spell: None,
        }
    }
}

/// A continuous effect from a resolved spell or ability: +power/+toughness for one object.
#[derive(Clone, Debug, PartialEq)]
pub struct Boost {
    pub target: ObjId,
    pub zcc: u32,
    pub power: i32,
    pub toughness: i32,
    /// ends in the cleanup step (otherwise, when the object leaves the battlefield)
    pub until_end_of_turn: bool,
    /// "until your next turn": ends as that player's turn begins
    pub until_turn_of: Option<Seat>,
    /// it changes the permanent even while it isn't a creature (`BoostAllEffect` boosts the objects it locked in:
    /// a Vehicle crewed as Mouth of the Storm's trigger resolved keeps its -3/-0)
    pub noncreature_too: bool,
}

/// What a game event is (`GameEventKind`, what the web client's effects play from)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventKind {
    Damage,
    LifeGain,
    LifeLoss,
    Draw,
    Zone,
    // what only the Career's tally counts (`CareerTally.Recorder`): the client gets none of these
    /// a turn begins: `player` is the active player (`BEGIN_TURN`)
    Turn,
    /// `player` cast `target` (`SPELL_CAST`)
    Cast,
    /// `source`'s effect countered `target`, `player`'s spell (`COUNTERED`)
    Countered,
    /// `player` activated `source`'s loyalty ability (`ACTIVATED_ABILITY` of a planeswalker)
    Loyalty,
    /// `source` became attached to `target` (`ATTACHED`)
    Attached,
    /// `player` sacrificed `target` (`SACRIFICED_PERMANENT`)
    Sacrificed,
    /// `player` discarded `target` (`DISCARDED_CARD`)
    Discarded,
    /// `player` milled `amount` cards (`MILLED_CARD`)
    Milled,
    /// `player` made `amount` mana (`MANA_ADDED`)
    Mana,
    /// `player` played a land (`LAND_PLAYED`)
    Land,
    /// `source` attacks (`ATTACKER_DECLARED`)
    Attack,
    /// `source` blocks (`BLOCKER_DECLARED`)
    Block,
    /// `player` put `amount` counters on `target` (`COUNTERS_ADDED`)
    Counters,
}

impl EventKind {
    /// whether the client gets it (`GameEventView`): the others are the Career tally's
    pub fn shown(self) -> bool {
        matches!(
            self,
            EventKind::Damage | EventKind::LifeGain | EventKind::LifeLoss | EventKind::Draw | EventKind::Zone
        )
    }
}

/// One game event (`GameEventView`): damage dealt, life gained or lost, a card drawn, a zone change
#[derive(Clone, Debug, PartialEq)]
pub struct GameEvent {
    pub kind: EventKind,
    pub target: Option<Target>,
    pub source: Option<ObjId>,
    pub player: Option<Seat>,
    pub amount: i32,
    pub from: Option<Zone>,
    pub to: Option<Zone>,
    pub combat: bool,
    /// the turn it happened on (`Game::record` sets it)
    pub turn: u32,
}

impl GameEvent {
    pub fn new(kind: EventKind) -> GameEvent {
        GameEvent {
            kind,
            target: None,
            source: None,
            player: None,
            amount: 0,
            from: None,
            to: None,
            combat: false,
            turn: 0,
        }
    }
}

/// A game: everything the rules look at. Cloning one is cheap (see the module docs).
#[derive(Clone, Debug)]
pub struct Game {
    pub rng: JavaRandom,
    /// each starting card's printing (`SET:number`), by object id, for its view (docs/rust/Views.md)
    pub printings: Arc<Vec<String>>,
    /// each player's life as the game began (`Game.getStartingLife`): 20, or the table's
    pub starting_life: i32,
    /// how each seat's pool pays (`ManaPool.autoPayment`, `autoPaymentRestricted`), two bits a seat: manual (only the
    /// type the player unlocks pays), unrestricted (all of the pool pays). Shared by the game's copies, so a player
    /// who changes it mid-game changes it for the game being played (the host's copy of it, as it asks, sets it)
    pub pool_modes: Arc<std::sync::atomic::AtomicU32>,
    pub turn: u32,
    /// day (true) or night (false), once a daybound or nightbound permanent appeared
    pub day: Option<bool>,
    pub step: Option<Step>,
    pub active: Option<Seat>,
    pub priority: Option<Seat>,
    pub players: Arc<Vec<Player>>,
    pub objs: Arc<Vec<Arc<Obj>>>,
    pub battlefield: Arc<Vec<ObjId>>,
    pub exile: Arc<Vec<ObjId>>,
    /// emblems, in the order they were made
    pub command: Arc<Vec<ObjId>>,
    /// bottom first: the top is the last
    pub stack: Arc<Vec<StackItem>>,
    pub boosts: Arc<Vec<Boost>>,
    /// triggered abilities waiting to be put on the stack, in the order they triggered
    pub pending: Vec<StackItem>,
    /// delayed triggers waiting for the end of combat
    pub at_end_of_combat: Vec<StackItem>,
    /// "at the beginning of the next end step" delayed triggers
    pub at_next_end_step: Vec<StackItem>,
    /// "when that creature dies this turn" delayed triggers: the creature as it was, the turn, whether its target is
    /// the creature's controller, the trigger
    pub when_dies: Vec<(ObjId, u32, u32, bool, StackItem)>,
    /// "at the beginning of the next turn's upkeep" delayed triggers
    pub at_next_upkeep: Vec<StackItem>,
    /// cards exiled by rebound: (card, its exile zcc, owner), offered at the owner's next upkeep
    pub rebound: Vec<(ObjId, u32, Seat)>,
    /// "when you next cast a creature spell this turn" delayed triggers: whose, the turn, and the trigger
    pub next_creature_spell: Vec<(Seat, u32, StackItem)>,
    /// "when you next cast an instant or sorcery spell this turn": the player, the turn, the delayed trigger, and the
    /// spells it waits for when they aren't instants and sorceries (`CastNextSpellDelayedTriggeredAbility`'s filter)
    pub next_instant_sorcery: Vec<(Seat, u32, StackItem, Option<crate::card::Filter>)>,
    /// "until your next turn, whenever one or more creatures attack one of your opponents" delayed triggers: whose,
    /// and the trigger (its `fixed` set to the attackers)
    pub attacks_opponent: Vec<(Seat, StackItem)>,
    /// spells that enter with more +1/+1 counters: the card as it was on the stack, and how many
    pub enter_counters: Vec<(ObjId, u32, i32)>,
    /// the stack position of the spell or ability being cast or activated: not paid for yet (a view shows it unpaid)
    pub unpaid: Option<usize>,
    /// `unpaid` once its mana is paid, while its other costs are (the view shows it paid)
    pub mana_paid: Option<usize>,
    /// this combat's groups, for views
    pub combat_groups: Vec<CombatGroup>,
    /// cards revealed while the object resolving resolves, for views
    pub revealed: Vec<Revealed>,
    /// cards looked at while the object resolving resolves, for the viewer's views
    pub looked_at: Vec<LookedAt>,
    /// the piles (`Cards` locals) of the card's own effect resolving now, by slot name (`EffectDef::Effects`)
    pub piles: Vec<(String, Vec<ObjId>)>,
    /// counts attachments, for their order; also the timestamp of permanents entering and of base power and
    /// toughness set as an effect resolves (`Game::timestamp`)
    pub attach_clock: u64,
    /// continuous effects that started as a spell or ability resolved and last until end of turn, with no object
    /// locked in (Java's `RestrictionEffect`s): the effect, its controller, its source
    pub eot_effects: Vec<(EffectDef, Seat, ObjId)>,
    /// static handlers' effects (`EffectDef::CardStatic`) that resolved, until the turn's cleanup
    pub card_statics: Vec<ResolvedStatic>,
    /// what's left of a static handler's amount (a prevention shield's), by its source and timestamp
    pub static_amounts: Vec<(ObjId, u64, i32)>,
    /// layer 7b until end of turn: the permanent, its zone change count, the base power and toughness, the timestamp
    pub base_pt_until_eot: Vec<(ObjId, u32, i32, i32, u64)>,
    /// "prevent the next N damage that would be dealt to target …" this turn: the target, its zone change count,
    /// what's left to prevent (`i32::MAX`: all), combat damage only, the effect's source
    pub prevent_to_target: Vec<(Target, u32, i32, bool, ObjId)>,
    /// abilities that resolved (`AbilityResolvedWatcher`): the source, the ability's index, the source's zone change
    /// count then, the turn
    pub abilities_resolved: Vec<(ObjId, usize, u32, u32)>,
    /// the step's start, before its priority round (Java's `StepPart.PRE`): declaring attackers shows nothing playable
    pub step_start: bool,
    /// `GameImpl.checkInfiniteLoop`'s state: the sources of the last resolved stack objects (up to four), the stack's
    /// size after the last resolution, the players' life when the count started, the count (reset at each priority
    /// round) and how many times in a row the players declined to call it a draw
    pub loop_sources: Vec<ObjId>,
    /// while a mana payment is asked: the spell can be paid with a special action (delve, convoke, improvise), the
    /// "Special" button (`GameView.special`)
    pub special_mana: bool,
    /// while a library is searched (`TargetCardInLibrary.choose`): the whole library as the client is shown it,
    /// sorted by name, and the cards chosen so far. The question's cards are the ones that can still be picked
    /// (Java's `possibleTargets`).
    pub searching: Option<(Vec<ObjId>, Vec<ObjId>)>,
    pub loop_stack: usize,
    pub loop_lives: Option<Vec<i32>>,
    pub loop_count: u32,
    pub loop_declined: u32,
    /// the spell whose mana cost is being paid (or checked): conditional mana can be spent only on some spells
    pub paying_for: std::cell::Cell<Option<ObjId>>,
    /// what's being paid for is an activated ability of `paying_for` (not a spell)
    pub paying_ability: std::cell::Cell<bool>,
    /// a spell cast while an effect resolves ("you may cast"): timing doesn't apply
    pub any_time: std::cell::Cell<bool>,
    /// paying a forage cost: its rule reads "forage" (no feel-bad word for the auto-choice)
    pub foraging: std::cell::Cell<bool>,
    /// the mana spent on payments so far, by kind (WUBRG, colorless): a spell's colors spent are read from it
    pub spent_tally: [u32; 6],
    /// of `spent_tally`, what paid a mana ability's own mana cost while paying for something else (Guild Globe's
    /// {2}): not spent on that spell, for its colors spent
    pub nested_spent: [u32; 6],
    /// "damage can't be prevented this turn" (Stomp): the turn it was said
    pub no_prevention_turn: Option<u32>,
    /// "prevent all damage that would be dealt to it this turn": the permanent, its zone-change count, the turn
    pub prevent_all_to: Vec<(ObjId, u32, u32)>,
    /// "prevent all combat damage a source of your choice would deal this turn": the source, its zone-change count,
    /// the turn
    pub prevent_combat_from: Vec<(ObjId, u32, u32)>,
    /// "players can't gain life this turn" (Skullcrack): the turn it was said
    pub no_lifegain_turn: Option<u32>,
    /// "until your next turn, you may cast spells of the filter as though they had flash": the player, the filter,
    /// the effect's source (Teferi, Time Raveler's +1)
    pub flash_until_next_turn: Vec<(Seat, crate::card::Filter, ObjId)>,
    /// the stack object resolving: where it was on the stack (from the bottom), and its card
    pub resolving: Option<(usize, ObjId)>,
    /// the stack object resolving, off the stack while it resolves (a view shows it where it was)
    pub resolving_item: Option<StackItem>,
    /// a triggered ability choosing its mode as it's put on the stack (not on it yet): the prompt's modes are its
    pub moding: Option<StackItem>,
    /// a card entering the battlefield from a hand while its as-enters choices are asked: Java moves it after
    /// them, so it's still in the hand, at this index (`ZonesHandler.moveCard`)
    pub entering_from_hand: Option<(ObjId, Seat, usize)>,
    /// decision logs name exiled cards in the order the cards were created (`DecisionLog.setStartOrder`), not the
    /// order they were exiled
    pub exile_by_creation: bool,
    /// a creature died this turn (`MorbidWatcher`)
    pub creature_died: bool,
    /// how many creatures died this turn (`CreaturesDiedWatcher`)
    pub creatures_died: u32,
    /// a nonland permanent left the battlefield or a spell was warped this turn (`VoidWatcher`)
    pub void_this_turn: bool,
    /// questions Java asked where the engine has no one to ask (`move_to`): which of several replacement effects
    /// applies, when they all do the same; asked before the next question or trace
    pub owed_choices: Vec<Seat>,
    /// the same, owed where the game can't be changed (`Game::triggers`: two or more Panharmonicons applying; two
    /// counter or token doublers)
    pub owed_replacement_choices: std::cell::RefCell<Vec<Seat>>,
    /// what the event triggers are checked for now is (`NumberOfTriggersEvent`'s entered and died permanents)
    pub trigger_cause: Option<TriggerCause>,
    /// the turn a resolving `ExileUncastCreaturesEffect` works until its end (Mistcaller)
    pub exile_uncast_turn: Option<u32>,
    /// a card entering whose replacement effects see it as it was (Enduring Glimmer's return: Java's type effect
    /// applies once it's there, so "creatures enter tapped" still taps it)
    pub enters_as: Option<(ObjId, Arc<CardDef>)>,
    /// extra turns to take, the last added first (`TurnMod`)
    pub extra_turns: Vec<Seat>,
    /// Emrakul, the Promised End: (player, controller) for a player's next turn, controlled by another
    pub control_next_turn: Vec<(Seat, Seat)>,
    /// (player, controller) while a player's turn is controlled by another; an extra turn follows it
    pub controlled_by: Vec<(Seat, Seat)>,
    /// "at the beginning of your first upkeep" delayed triggers (Devourer of Destiny's), each its controller's
    pub at_first_upkeep: Vec<StackItem>,
    pub grants: Arc<Vec<Grant>>,
    /// abilities lost until end of turn: (permanent, its zcc, the ability, the turn)
    pub lost_until_eot: Vec<(ObjId, u32, AbilityDef, u32)>,
    /// what made a permanent block this turn if able, with the permanent's zone change count (its hint names it)
    pub must_block_from: Vec<(ObjId, u32, ObjId)>,
    /// the game log's lines as Java writes them (`informPlayers`), while it's kept (replays and the AI's copy compare
    /// it with Java's; docs/rust/Views.md)
    pub log: Option<Vec<String>>,
    /// the game's events since they were last taken, while they're kept (a hosted game's views carry them)
    pub events: Option<Vec<GameEvent>>,
    /// permanents exiled instead if they would die this turn
    pub exile_if_dies: Vec<(ObjId, u32)>,
    /// cards exiled instead if they would be put into a graveyard this turn (`ThatSpellGraveyardExileReplacementEffect`:
    /// Torrential Gearhulk's instant, from anywhere)
    pub exile_if_graveyard: Vec<ObjId>,
    /// control changes with a duration, oldest first
    pub control_changes: Vec<ControlChange>,
    /// creatures dealt damage this turn, by source (`DamagedByWatcher`): the source, the creature, its zone change
    /// count (Anger of the Gods exiles each one it damaged)
    pub damaged_by_source: Vec<(ObjId, ObjId, u32)>,
    /// players dealt combat damage this turn, by source (Steel Hellkite's watcher): the source, the player
    pub combat_damaged_players: Vec<(ObjId, Seat)>,
    /// the creatures that attacked this turn, as they were (`AttackedThisTurnWatcher.getAttackedThisTurnCreatures`)
    pub attacked_this_turn: Vec<(ObjId, u32)>,
    /// the players whose lands have all basic land types until end of turn (Energybending)
    pub all_basics_until_eot: Vec<Seat>,
    /// subtypes permanents have until end of turn (Sarkhan, Dragon Ascendant's "becomes a Dragon"): the permanent,
    /// its zone change count, the subtype
    pub subtypes_until_eot: Vec<(ObjId, u32, String)>,
    /// subtypes permanents have for as long as they stay (amass's `AddCardSubTypeTargetEffect`, `Duration.Custom`)
    pub subtypes_while_there: Vec<(ObjId, u32, String)>,
    /// permanents that don't untap during their controller's next untap step (and their zone change count)
    pub skip_next_untap: Vec<(ObjId, u32)>,
    /// `DontUntapInControllersUntapStepTargetEffect`: a permanent (its zone change count) that doesn't untap while its
    /// source (its zone change count) stays on the battlefield under the controller it had (`Some`,
    /// `WhileControlled`), or (None, `DontUntapAsLongAsSourceTappedEffect`) while the source stays tapped
    pub dont_untap_while: Vec<(ObjId, u32, ObjId, u32, Option<Seat>)>,
    /// `CanBlockAdditionalCreatureTargetEffect` this turn: the creature, its zone change count, how many more it can
    /// block (0: any number)
    pub extra_blocks: Vec<(ObjId, u32, i32)>,
    /// combat requirements resolved this turn: what (its zone change count) must be blocked by all able, must attack,
    /// or must block (the attacker named, with its zone change count), and whether it lasts only this combat
    pub requirements: Vec<Requirement>,
    /// extra phases to come (Java's turn mods, kept from turn to turn as Java keeps them): whose turn, the phase
    /// (its first step), the phase it comes after (None: the current one), and whether an extra main phase follows it
    /// (`AddCombatAndMainPhaseEffect`); the last one added is used first
    pub extra_phases: Vec<(Seat, Step, Option<Step>, bool)>,
    /// while an extra phase is played: the step the turn goes on with after it (and the extra phases after it)
    pub resume_step: Option<Step>,
    /// this turn's own combat phase is over (`FirstCombatPhaseCondition`: an extra one doesn't count)
    pub combat_phase_done: bool,
    /// `DontUntapInControllersUntapStepAllEffect` until your next turn: its controller, "you" or "any", the filter,
    /// the source and the turn it resolved; it applies in the next untap step of that player's turn
    pub dont_untap_next: Vec<(Seat, String, Filter, ObjId, u32)>,
    /// triggered abilities with a limit each turn that triggered this turn: source, its zone changes, ability index
    pub triggered_this_turn: Vec<(ObjId, u32, usize)>,
    /// while triggered abilities go on the stack: what their targets trigger (ward, valiant), which Java's
    /// `addSimultaneousEvent` holds until every player's are on (`GameImpl.checkTriggered`)
    pub targeted_later: Option<Vec<StackItem>>,
    /// permanents sacrificed together: where in `pending` the next one's sacrifice triggers go (before what their
    /// leaving triggers)
    pub sac_insert: Option<usize>,
    /// the cards an effect is putting onto the battlefield at once, in order (they see each other enter)
    pub entering_together: Vec<ObjId>,
    /// the color the last "any color" mana was made in without asking (one color owed): Java asks it
    pub any_color_made: Option<crate::card::Color>,
    /// combat damage's sources' deathtouch and lifelink as the step started: Java doesn't apply continuous effects
    /// again while the damage is dealt (Skrelv's Hive's corrupted lifelink, the poison counters coming with it)
    pub combat_keywords: Vec<(ObjId, bool, bool)>,
    /// activated abilities with a limit each turn activated this turn: source, ability index
    pub activated_this_turn: Vec<(ObjId, usize)>,
    /// the battlefield as the current action began, for the dies triggers of what it moves all at once
    pub lookback: Vec<ObjId>,
    /// what was sacrificed since Java last reset its short-living LKI (`resetShortLivingLKI`)
    pub short_lki: Vec<ObjId>,
    /// Etchings of Kumano that left the battlefield, with their controller and the turn: one leaving in the same
    /// action as a creature still exiles it, though its card shows the saga by then
    pub etchings_left: Vec<(ObjId, Seat, u32)>,
    /// cards exiled until a permanent (and its zone change count) leaves the battlefield
    pub exiled_until_leaves: Vec<(ObjId, u32, ObjId)>,
    /// the last token class made, as `token_image` picked its image: name, class, set, image; the tokens of it
    /// that enter take it
    pub token_made: Option<(String, String, String, u32)>,
    /// earthbent lands (with their zone change counts): when one dies or is exiled, it returns tapped
    pub earthbent: Vec<(ObjId, u32)>,
    /// cards exiled into a permanent's own exile zone (its id and zone change count), that stay there
    pub exiled_for_source: Vec<(ObjId, u32, ObjId)>,
    /// cards to return now: their exiling permanent left (`OnLeaveReturnExiledAbility`, which uses no stack), with
    /// the seat whose trigger that is
    pub returning: Vec<(Seat, ObjId)>,
    /// damage permanents were dealt since triggers were last checked, in order (`DAMAGED_BATCH_FOR_ONE_PERMANENT`)
    pub damaged_batch: Vec<(ObjId, i32)>,
    /// damage players were dealt since triggers were last checked: source, player, amount, combat
    /// (`DAMAGED_BATCH_FOR_ONE_PLAYER`)
    pub player_damage_batch: Vec<(ObjId, Seat, i32, bool)>,
    /// damage dealt since triggers were last checked: source, amount, combat (`DAMAGED_BATCH_BY_SOURCE`)
    pub source_damage_batch: Vec<(ObjId, i32, bool)>,
    /// mana abilities activated so far: each one's damage is its own batch (`processManaAction`)
    pub mana_batch: u64,
    /// "opponents are dealt noncombat damage" sources that triggered, with the mana batch they triggered in
    pub noncombat_fired: Vec<(ObjId, u64)>,
    /// "one or more … enter" triggers since triggers were last checked, after the others (`ZONE_CHANGE_BATCH`)
    pub batch_pending: Vec<StackItem>,
    /// a card was put into exile from a graveyard or the battlefield since triggers were last checked (Ketramose)
    pub exiled_batch: bool,
    /// the cards that left a graveyard since triggers were last checked (`ZONE_CHANGE_BATCH`)
    pub left_graveyard: Vec<ObjId>,
    /// cards discarded since the last `DISCARDED_CARDS` (`Game::discarded_cards`)
    pub discarded: Vec<(Seat, ObjId)>,
    /// `LOST_LIFE_BATCH_FOR_ONE_PLAYER` triggers waiting for the batch: the player who lost life, and the trigger
    /// with the total as its X
    pub life_batch: Vec<(Seat, StackItem)>,
    /// the spell resolving and its controller, after it's off the stack
    pub resolving_controller: Option<(ObjId, Seat)>,
    /// while a `not_first` target is chosen: the ability's targets chosen before it
    pub chosen_before: Vec<Target>,
    /// lands that are a basic land type instead of their own for as long as a permanent stays (Tide Shaper): the land
    /// and its zone change count, the source and its count, the type
    pub land_becomes: Vec<(ObjId, u32, ObjId, u32, String)>,
    /// cards exiled until something leaves that go back to their owner's hand, not the battlefield (Kitesail
    /// Freebooter's)
    pub return_to_hand: Vec<ObjId>,
    /// this combat has a first-strike damage step
    pub first_strike_step: bool,
    /// `FirstStrikeWatcher`: the creatures that dealt damage in this combat's first-strike step (702.7c: only double
    /// strike deals damage again)
    pub first_strikers: Vec<ObjId>,
    /// the game is over (a player lost, or it reached its last turn)
    pub over: bool,
    /// what Java's per-turn watchers saw this turn (`TurnWatch`), cleared as each turn begins
    pub watch: TurnWatch,
}

/// A spell cast this turn, as `SpellsCastWatcher` keeps it: its characteristics as it was cast
#[derive(Clone, Debug, PartialEq)]
pub struct SpellSeen {
    pub caster: Seat,
    pub card: ObjId,
    pub from: Option<Zone>,
    pub creature: bool,
    /// artifact, legendary or Saga (`HistoricPredicate`)
    pub historic: bool,
}

/// What Java's watchers that reset each turn saw this turn, for the conditions, values and predicates that read them
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TurnWatch {
    /// `SpellsCastWatcher`, in the order they were cast
    pub spells: Vec<SpellSeen>,
    /// `PermanentsSacrificedWatcher`: who sacrificed, and whether it was an artifact
    pub sacrificed: Vec<(Seat, bool)>,
    /// `CardsLeftGraveyardWatcher`: the owner of each card that left a graveyard
    pub left_graveyard: Vec<Seat>,
    /// `DiscardedCardWatcher`: who discarded, once a card
    pub discarded: Vec<Seat>,
    /// `ScryOrSurveilWatcher`: who scried or surveilled
    pub scried: Vec<Seat>,
    /// `CardsPutIntoGraveyardWatcher`: each card put into a graveyard, its zone-change count there, and whether it
    /// came from the battlefield
    pub to_graveyard: Vec<(ObjId, u32, bool)>,
    /// `CrewedVehicleWatcher`: (vehicle, its zone-change count, a creature that crewed it, its count)
    pub crewed: Vec<(ObjId, u32, ObjId, u32)>,
    /// `LifeLostThisTurnWatcher`: a `LOST_LIFE_BATCH_FOR_ONE_PLAYER` for each, once a batch
    pub life_loss_batches: Vec<Seat>,
    /// the players who lost life since the last batch
    pub life_loss_unbatched: Vec<Seat>,
    /// `CommittedCrimeWatcher`: who committed a crime
    pub crimes: Vec<Seat>,
    /// `CreaturesDiedWatcher`: the controller of each creature that died
    pub died: Vec<Seat>,
    /// each permanent put into a graveyard from the battlefield (a dies event, `ZoneChangeEvent.isDiesEvent`), as
    /// (its controller, an artifact, a creature, a token): Ichor Shade's and Gadrak's watchers
    pub dies: Vec<(Seat, bool, bool, bool)>,
    /// `PermanentsEnteredBattlefieldWatcher` and the like: each permanent that entered, as (its controller, an
    /// artifact, a creature, a Goblin)
    pub entered: Vec<(Seat, bool, bool, bool)>,
    /// the same permanents, with their zone-change counts as they entered and their controllers then
    /// (`PermanentsEnteredBattlefieldWatcher.getThisTurnEnteringPermanents`)
    pub entered_permanents: Vec<(ObjId, u32, Seat)>,
    /// `COUNTER_ADDED` for +1/+1 counters: who put them, and how many (Iridescent Hornbeetle's watcher)
    pub p1p1_put: Vec<(Seat, i32)>,
    /// `CardsExiledThisTurnWatcher`: objects put into exile
    pub exiled: u32,
    /// `TriggeredAbility.setDidThisTurn`: the "do this only once each turn" abilities done this turn (the source, its
    /// zone-change count, the ability's index)
    pub did_this_turn: Vec<(ObjId, u32, usize)>,
    /// `CastFromGraveyardOnceWatcher`: the permanents (and their zone-change counts) whose "once during each of your
    /// turns, cast from your graveyard" was used this turn
    pub graveyard_once: Vec<(ObjId, u32)>,
}

impl Game {
    /// a snow permanent (`SuperType.SNOW`): its mana pays snow symbols
    pub fn is_snow(&self, id: ObjId) -> bool {
        self.obj(id).def.supertypes.iter().any(|s| s == "SNOW")
    }

    pub fn new(seats: usize, seed_state: i64) -> Game {
        let mut rng = JavaRandom::new(0);
        rng.set_state(seed_state);
        Game {
            rng,
            starting_life: 20,
            pool_modes: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            turn: 0,
            step: None,
            active: None,
            priority: None,
            players: Arc::new((0..seats).map(|_| Player { life: 20, ..Player::default() }).collect()),
            objs: Arc::new(Vec::new()),
            battlefield: Arc::new(Vec::new()),
            command: Arc::new(Vec::new()),
            exile: Arc::new(Vec::new()),
            stack: Arc::new(Vec::new()),
            boosts: Arc::new(Vec::new()),
            pending: Vec::new(),
            at_end_of_combat: Vec::new(),
            at_next_end_step: Vec::new(),
            when_dies: Vec::new(),
            at_next_upkeep: Vec::new(),
            rebound: Vec::new(),
            attacks_opponent: Vec::new(),
            next_creature_spell: Vec::new(),
            next_instant_sorcery: Vec::new(),
            enter_counters: Vec::new(),
            unpaid: None,
            mana_paid: None,
            combat_groups: Vec::new(),
            revealed: Vec::new(),
            looked_at: Vec::new(),
            piles: Vec::new(),
            attach_clock: 0,
            eot_effects: Vec::new(),
            card_statics: Vec::new(),
            static_amounts: Vec::new(),
            base_pt_until_eot: Vec::new(),
            prevent_to_target: Vec::new(),
            abilities_resolved: Vec::new(),
            step_start: false,
            loop_sources: Vec::new(),
            special_mana: false,
            searching: None,
            loop_stack: 0,
            loop_lives: None,
            loop_count: 0,
            loop_declined: 0,
            paying_for: std::cell::Cell::new(None),
            paying_ability: std::cell::Cell::new(false),
            any_time: std::cell::Cell::new(false),
            foraging: std::cell::Cell::new(false),
            spent_tally: [0; 6],
            nested_spent: [0; 6],
            no_prevention_turn: None,
            prevent_all_to: Vec::new(),
            prevent_combat_from: Vec::new(),
            no_lifegain_turn: None,
            flash_until_next_turn: Vec::new(),
            resolving: None,
            resolving_item: None,
            moding: None,
            entering_from_hand: None,
            exile_by_creation: false,
            creature_died: false,
            creatures_died: 0,
            void_this_turn: false,
            owed_choices: Vec::new(),
            owed_replacement_choices: Default::default(),
            trigger_cause: None,
            exile_uncast_turn: None,
            enters_as: None,
            extra_turns: Vec::new(),
            control_next_turn: Vec::new(),
            controlled_by: Vec::new(),
            at_first_upkeep: Vec::new(),
            grants: Arc::new(Vec::new()),
            lost_until_eot: Vec::new(),
            must_block_from: Vec::new(),
            log: None,
            events: None,
            exile_if_dies: Vec::new(),
            exile_if_graveyard: Vec::new(),
            control_changes: Vec::new(),
            damaged_by_source: Vec::new(),
            combat_damaged_players: Vec::new(),
            attacked_this_turn: Vec::new(),
            all_basics_until_eot: Vec::new(),
            subtypes_until_eot: Vec::new(),
            subtypes_while_there: Vec::new(),
            skip_next_untap: Vec::new(),
            dont_untap_while: Vec::new(),
            extra_blocks: Vec::new(),
            requirements: Vec::new(),
            extra_phases: Vec::new(),
            resume_step: None,
            combat_phase_done: false,
            dont_untap_next: Vec::new(),
            triggered_this_turn: Vec::new(),
            targeted_later: None,
            sac_insert: None,
            entering_together: Vec::new(),
            any_color_made: None,
            combat_keywords: Vec::new(),
            activated_this_turn: Vec::new(),
            lookback: Vec::new(),
            short_lki: Vec::new(),
            printings: Arc::new(Vec::new()),
            etchings_left: Vec::new(),
            exiled_until_leaves: Vec::new(),
            token_made: None,
            earthbent: Vec::new(),
            exiled_for_source: Vec::new(),
            returning: Vec::new(),
            day: None,
            damaged_batch: Vec::new(),
            player_damage_batch: Vec::new(),
            source_damage_batch: Vec::new(),
            mana_batch: 0,
            noncombat_fired: Vec::new(),
            batch_pending: Vec::new(),
            exiled_batch: false,
            left_graveyard: Vec::new(),
            discarded: Vec::new(),
            life_batch: Vec::new(),
            resolving_controller: None,
            chosen_before: Vec::new(),
            land_becomes: Vec::new(),
            return_to_hand: Vec::new(),
            first_strike_step: false,
            first_strikers: Vec::new(),
            over: false,
            watch: TurnWatch::default(),
        }
    }

    pub fn seats(&self) -> usize {
        self.players.len()
    }

    pub fn obj(&self, id: ObjId) -> &Obj {
        &self.objs[id.0 as usize]
    }

    pub fn obj_mut(&mut self, id: ObjId) -> &mut Obj {
        Arc::make_mut(&mut Arc::make_mut(&mut self.objs)[id.0 as usize])
    }

    pub fn player(&self, seat: Seat) -> &Player {
        &self.players[seat]
    }

    pub fn player_mut(&mut self, seat: Seat) -> &mut Player {
        &mut Arc::make_mut(&mut self.players)[seat]
    }

    pub fn opponent(&self, seat: Seat) -> Seat {
        (seat + 1) % self.seats()
    }

    /// Creates a card owned by `owner` in `zone`, without putting it in that zone's list.
    pub fn create_obj(&mut self, def: Arc<CardDef>, owner: Seat, zone: Zone) -> ObjId {
        let id = ObjId(self.objs.len() as u32);
        Arc::make_mut(&mut self.objs).push(Arc::new(Obj {
            id,
            zcc: 0,
            def,
            owner,
            controller: owner,
            zone,
            adventure_zone: zone,
            harmonize_tap: None,
            tapped: false,
            damage: 0,
            counters: Vec::new(),
            lki_counters: Vec::new(),
            lki_keywords: Vec::new(),
            lki_attached_to: None,
            lki_attachments: Vec::new(),
            lki_def: None,
            face_down: false,
            face_down_as: None,
            face_up: None,
            attacking: None,
            attacking_pw: None,
            protector: None,
            loyalty_used: false,
            kicked: false,
            cast: false,
            cast_lki: false,
            toughness_lki: None,
            power_lki: None,
            p1p1_added: false,
            flashback_turn: None,
            solved: false,
            until_end_of_turn_def: None,
            becomes_base: None,
            becomes_auras: Vec::new(),
            land_type_base: None,
            chosen_type: None,
            info: Vec::new(),
            chosen_name: None,
            prepared_copy: None,
            prepared_for: None,
            exiled_with: None,
            transformed_in_place: false,
            doors: [false; 2],
            room_cast: None,
            fused_cast: false,
            foretold_turn: None,
            plotted_turn: None,
            suspended: false,
            cast_from: None,
            uncounterable: false,
            printed_def: None,
            pre_guildpact: None,
            guildpact_order: Vec::new(),
            copied_def: None,
            eot_copy: None,
            cast_x: 0,
            attached_to: None,
            adventurer: None,
            front: None,
            flashback: false,
            escaped: false,
            ninjutsu_defender: None,
            ninjutsu_pw: None,
            chosen: None,
            class_level: 1,
            lki_class_level: 1,
            exerted: false,
            powered_up: false,
            marks: Vec::new(),
            creature_types: None,
            base_pt: None,
            base_pt_at: 0,
            entered_at: 0,
            regen_shields: 0,
            exile_instead_of_dying: false,
            unearthed: false,
            damaged_by: (0, 0),
            lost_abilities: None,
            may_play: None,
            may_play_end_step: false,
            may_play_while: None,
            any_mana: false,
            play_free: false,
            cast_only: false,
            play_tax: 0,
            warped: false,
            evoked: false,
            impending: false,
            dashed: false,
            spent_colors: [0; 5],
            blitzed: false,
            offspring: false,
            warp_from: None,
            mana_spent: 0,
            entered_turn: 0,
            targeted: (0, 0),
            on_adventure: false,
            attached_at: 0,
            attached_key: 0,
            blocking: Vec::new(),
            blocks: 0,
            blocked: false,
            blocked_by: Vec::new(),
            sick: false,
            token_image: None,
            deathtouched: false,
            token: false,
            trigger_order: id.0 as u64,
            class_gained: Vec::new(),
        }));
        id
    }

    /// Whether `id` passes `filter`, as seen from `source` (whose controller is "you"; for targets, the chooser is
    /// passed as `you`). Works for objects in any zone: a spell's or a card's characteristics are its card's.
    pub fn filter_matches(&self, filter: &Filter, you: Seat, source: Option<ObjId>, id: ObjId) -> bool {
        filter.0.iter().all(|p| self.pred_holds(p, you, source, id))
    }

    fn pred_holds(&self, p: &Pred, you: Seat, source: Option<ObjId>, id: ObjId) -> bool {
        let o = self.obj(id);
        match p {
            Pred::Type { t } => self.has_type(id, *t),
            Pred::Subtype { s } => self.has_subtype(id, s),
            Pred::Supertype { s } => o.def.supertypes.iter().any(|x| x == s),
            Pred::Color { c } => self.colors(id).contains(c),
            Pred::Controller { who } => (o.controller == you) == (who == "you"),
            Pred::Owner { who } => (o.owner == you) == (who == "you"),
            Pred::Another => Some(id) != source,
            Pred::ControlledByFirstTarget => match self.chosen_before.first() {
                Some(Target::Player(p)) => o.controller == *p,
                Some(Target::Obj(x)) => o.controller == self.obj(*x).controller && id != *x,
                _ => true,
            },
            Pred::Multicolored => self.colors(id).len() > 1,
            Pred::Colorless => self.colors(id).is_empty(),
            Pred::NoManaSpent => self.obj(id).mana_spent == 0,
            Pred::KickedSpell => self.stack.iter().any(|i| i.card == id && i.ability.is_none() && i.kicked),
            Pred::TargetsPermanent { filter } => self.stack.iter().any(|i| {
                i.card == id
                    && i.ability.is_none()
                    && i.targets.iter().flatten().any(|t| match t {
                        Target::Obj(x) => {
                            self.obj(*x).zone == Zone::Battlefield && self.filter_matches(filter, you, source, *x)
                        }
                        _ => false,
                    })
            }),
            Pred::ManaCostIn { costs } => costs.iter().any(|c| *c == o.def.cost.text),
            Pred::ManaValueAtMostSourcePower => {
                source.is_some_and(|src| {
                    // a dies trigger's source (Gurgling Anointer): its power as it last was on the battlefield
                    let s = self.obj(src);
                    let power = match s.power_lki {
                        Some(p) if s.zone != Zone::Battlefield => p,
                        _ => self.power_toughness(src).0,
                    };
                    o.def.mana_value() as i32 <= power
                })
            }
            Pred::ManaValueEqualSourceCounters { counter } => source.is_some_and(|src| {
                let s = self.obj(src);
                let n = if s.zone == Zone::Battlefield {
                    s.counter(counter)
                } else {
                    s.lki_counters.iter().filter(|(k, _)| k == counter).map(|(_, n)| *n).sum()
                };
                o.def.mana_value() as i32 == n
            }),
            Pred::AuraAttached => self
                .battlefield
                .iter()
                .any(|a| Some(*a) != source && self.attached(*a) == Some(id) && self.has_subtype(*a, "Aura")),
            Pred::ChosenName => {
                source.and_then(|src| self.obj(src).chosen_name.as_ref()).is_some_and(|n| *n == self.obj(id).def.name)
            }
            Pred::Adventure => self.obj(id).def.adventure.is_some() || self.obj(id).adventurer.is_some(),
            Pred::Mv { cmp, n } => cmp.holds(o.def.mana_value() as i32, *n),
            Pred::Power { cmp, n } => cmp.holds(self.power_toughness(id).0, *n),
            Pred::TotalPowerToughness { n } => {
                let (p, t) = self.power_toughness(id);
                p + t <= *n
            }
            Pred::Toughness { cmp, n } => cmp.holds(self.power_toughness(id).1, *n),
            Pred::Tapped { v } => o.tapped == *v,
            // a target adjuster: not applied while checking what's playable (the spell not yet cast)
            Pred::ManaValueX { cmp } => {
                match source.and_then(|s| self.stack.iter().rev().find(|i| i.card == s && i.ability.is_none())) {
                    Some(i) => cmp.holds(o.def.mana_value() as i32, i.x),
                    None => true,
                }
            }
            Pred::AnotherTarget => !self.chosen_before.contains(&Target::Obj(id)),
            Pred::BlockingOrBlockedBySource { blocking, blocked_by } => source.is_some_and(|src| {
                (*blocking && o.blocking.contains(&src)) || (*blocked_by && self.obj(src).blocking.contains(&id))
            }),
            // the same, with the X of the spell or ability whose target it is
            Pred::PowerX { cmp } | Pred::ToughnessX { cmp } => {
                match source.and_then(|s| self.stack.iter().rev().find(|i| i.card == s)) {
                    Some(i) => {
                        let (power, toughness) = self.power_toughness(id);
                        cmp.holds(if matches!(p, Pred::PowerX { .. }) { power } else { toughness }, i.x)
                    }
                    None => true,
                }
            }
            Pred::CastFrom { zone } => {
                let z = if o.zone == Zone::Stack { o.cast_from } else { Some(o.zone) };
                z == Some(match zone.as_str() {
                    "graveyard" => Zone::Graveyard,
                    "exile" => Zone::Exile,
                    _ => Zone::Hand,
                })
            }
            Pred::Attacking => o.attacking.is_some(),
            Pred::PowerLessThanSource => {
                source.is_some_and(|src| self.power_toughness(id).0 < self.power_toughness(src).0)
            }
            Pred::InCombat => o.attacking.is_some() || !o.blocking.is_empty(),
            Pred::Token { v } => o.token == *v,
            Pred::FaceDown => o.face_down,
            Pred::ChosenSubtype { v } => {
                source.and_then(|s| self.obj(s).chosen_type.as_ref()).is_some_and(|t| self.has_subtype(id, t)) == *v
            }
            Pred::Counter { name, n } => o.counter(name) >= (*n).max(1) as i32,
            Pred::Ability { a } => self.has_ability(id, a),
            Pred::AbilityClass { class } => {
                let named = |a: &AbilityDef| format!("{a:?}").split([' ', '{', '(']).next() == Some(class.as_str());
                // (its own and those resolved effects gave it: a static's would loop back here, Slaughter Singer's
                // "creatures you control with toxic")
                let o = self.obj(id);
                o.def.abilities.iter().any(named)
                    || (o.zone == Zone::Battlefield
                        && self.grants.iter().any(|g| g.target == id && g.zcc == o.zcc && named(&g.ability)))
            }
            Pred::Name { n } => o.name() == n,
            Pred::PermanentCard => o.def.is_permanent_card(),
            Pred::Monocolored => self.colors(id).len() == 1,
            Pred::CounterAny => o.counters.iter().any(|(_, n)| *n > 0),
            Pred::EnteredThisTurn => o.zone == Zone::Battlefield && o.entered_turn == self.turn,
            Pred::ToGraveyardThisTurn { battlefield } => {
                o.zone == Zone::Graveyard
                    && self
                        .watch
                        .to_graveyard
                        .iter()
                        .any(|(c, zcc, from)| *c == id && *zcc == o.zcc && (*from || !battlefield))
            }
            Pred::CrewedSourceThisTurn => source.is_some_and(|src| {
                let s = self.obj(src);
                self.watch.crewed.iter().any(|(v, vz, c, cz)| *v == src && *vz == s.zcc && *c == id && *cz == o.zcc)
            }),
            Pred::Equipped => self.attachments(id).any(|a| self.has_subtype(a, "Equipment")),
            Pred::Enchanted => self.attachments(id).any(|a| self.has_subtype(a, "Aura")),
            Pred::Modified => {
                o.counters.iter().any(|(_, n)| *n > 0)
                    || self.attachments(id).any(|a| {
                        self.has_subtype(a, "Equipment")
                            || (self.has_subtype(a, "Aura") && self.obj(a).controller == o.controller)
                    })
            }
            Pred::DamagedThisTurn => o.damaged_by.0 == self.turn && o.damaged_by.1 != 0,
            Pred::Blocking => !o.blocking.is_empty(),
            Pred::ManaSpent { cmp, n } => cmp.holds(o.mana_spent as i32, *n),
            Pred::Card { handler } => (handler.f())(self, you, source, id),
            Pred::Not { x } => !self.pred_holds(x, you, source, id),
            Pred::Or { xs } => xs.iter().any(|x| self.pred_holds(x, you, source, id)),
            Pred::And { xs } => xs.iter().all(|x| self.pred_holds(x, you, source, id)),
        }
    }

    /// Whether an object has a card type now.
    /// whether it has Etching of Kumano's replacement effect
    fn is_etching(&self, id: ObjId) -> bool {
        self.obj(id).def.abilities.iter().any(|a| match a {
            AbilityDef::SimpleStaticAbility { effects, .. } => {
                effects.iter().any(|e| matches!(e, EffectDef::EtchingOfKumanoReplacementEffect))
            }
            _ => false,
        })
    }

    pub fn has_type(&self, id: ObjId, t: CardType) -> bool {
        self.obj(id).is(t)
    }

    /// Whether an object has a subtype now.
    /// The subtypes effects add to a permanent (`AddCardSubTypeSourceEffect`): its own static ones whose condition
    /// holds (Skyknight Squire's Knight while it has three counters), and those until end of turn.
    pub fn added_subtypes(&self, id: ObjId) -> Vec<&str> {
        let o = self.obj(id);
        let mut added: Vec<&str> = Vec::new();
        if o.zone != Zone::Battlefield {
            return added;
        }
        for a in &o.def.abilities {
            let AbilityDef::SimpleStaticAbility { effects, .. } = a else { continue };
            for e in effects {
                let (e, holds) = match e {
                    EffectDef::ConditionalContinuousEffect { effect, condition } => (&**effect, Some(condition)),
                    e => (e, None),
                };
                if let EffectDef::AddCardSubTypeSourceEffect { subtype } = e {
                    if !subtype.is_empty()
                        && !added.contains(&subtype.as_str())
                        && (holds.is_none() || self.condition_holds(holds, o.controller, id))
                    {
                        added.push(subtype);
                    }
                }
            }
        }
        for src in self.battlefield.iter() {
            if self.attached(*src) != Some(id) {
                continue;
            }
            for a in &self.obj(*src).def.abilities {
                let AbilityDef::SimpleStaticAbility { effects, .. } = a else { continue };
                for e in effects {
                    if let EffectDef::AddCardSubtypeAttachedEffect { subtype } = e {
                        if !added.contains(&subtype.as_str()) {
                            added.push(subtype);
                        }
                    }
                }
            }
        }
        for (x, zcc, t) in self.subtypes_until_eot.iter().chain(&self.subtypes_while_there) {
            if *x == id && *zcc == o.zcc && !added.contains(&t.as_str()) {
                added.push(t);
            }
        }
        added
    }

    pub fn has_subtype(&self, id: ObjId, s: &str) -> bool {
        let o = self.obj(id);
        if o.zone == Zone::Battlefield
            && (self.added_subtypes(id).contains(&s) || crate::rules::cards::has_subtype(self, id, s))
        {
            return true;
        }
        let def = &o.def;
        if let Some(types) = o.creature_types.as_ref().filter(|_| o.zone == Zone::Battlefield) {
            return types.iter().any(|x| x == s)
                || (crate::card::NON_CREATURE_TYPES.contains(&s) && def.subtypes.iter().any(|x| x == s));
        }
        def.subtypes.iter().any(|x| x == s)
            || (def.all_creature_types && !crate::card::NON_CREATURE_TYPES.contains(&s))
            || (o.zone == Zone::Battlefield
                && self.any_static(|g, src, e| match e {
                    EffectDef::AddCardSubtypeAllEffect { filter, subtype } => {
                        subtype == s && g.filter_matches(filter, g.obj(src).controller, Some(src), id)
                    }
                    _ => false,
                }))
    }

    /// Whether a permanent has an ability: printed, given by a resolved effect, or given by a static ability.
    /// The ward abilities statics give `id` (Coppercoat Vanguard's "each other Human you control has ward {1}")
    pub fn granted_wards(&self, id: ObjId) -> Vec<AbilityDef> {
        let mut out = Vec::new();
        for src in self.battlefield.iter() {
            let s = self.obj(*src);
            for a in &s.def.abilities {
                let AbilityDef::SimpleStaticAbility { effects, .. } = a else { continue };
                for e in effects {
                    if let EffectDef::GainAbilityControlledEffect { ability, .. }
                    | EffectDef::GainAbilityAttachedEffect { ability } = e
                    {
                        if matches!(**ability, AbilityDef::WardAbility { .. })
                            && self.static_gives(e, *src, s.controller, id, ability)
                        {
                            out.push((**ability).clone());
                        }
                    }
                }
            }
        }
        out
    }

    /// Protection: `id` has protection from a color of `source`
    pub fn protected_from(&self, id: ObjId, source: ObjId) -> bool {
        self.colors(source).iter().any(|c| self.has_ability(id, &AbilityDef::ProtectionFromColorAbility { c: *c }))
            || self.obj(id).def.abilities.iter().chain(self.granted_abilities(id).iter()).any(|a| match a {
                AbilityDef::ProtectionAbility { filter } => {
                    self.filter_matches(filter, self.obj(id).controller, Some(id), source)
                }
                _ => false,
            })
    }

    /// The abilities a permanent has that it doesn't print, in the order they apply: its counters' keywords, resolved
    /// effects' (as they resolved), then static abilities' (by source), each once
    pub fn granted_abilities(&self, id: ObjId) -> Vec<AbilityDef> {
        self.granted(id, true)
    }

    /// `granted_abilities` with each grant as many times as it's given, the ones it prints too (two Auras' "toxic 2"
    /// are two abilities in Java, each adding its poison: Necrogen Communion)
    pub fn granted_instances(&self, id: ObjId) -> Vec<AbilityDef> {
        self.granted(id, false)
    }

    fn granted(&self, id: ObjId, once: bool) -> Vec<AbilityDef> {
        let o = self.obj(id);
        let mut out: Vec<AbilityDef> = Vec::new();
        let add = |a: &AbilityDef, out: &mut Vec<AbilityDef>| {
            if !once || !o.has(a) && !out.contains(a) {
                out.push(a.clone());
            }
        };
        for (counter, a) in [
            ("flying", AbilityDef::FlyingAbility),
            ("deathtouch", AbilityDef::DeathtouchAbility),
            ("indestructible", AbilityDef::IndestructibleAbility),
        ] {
            if o.counter(counter) > 0 {
                add(&a, &mut out);
            }
        }
        if o.zone != Zone::Battlefield {
            return out;
        }
        for g in self.grants.iter().filter(|g| g.target == id && g.zcc == o.zcc) {
            add(&g.ability, &mut out);
        }
        let lost_at = self.abilities_lost_at(id);
        for src in self.battlefield.iter().chain(self.command.iter()) {
            let s = self.obj(*src);
            if lost_at.is_some_and(|at| self.static_at(*src) < at) {
                continue;
            }
            for a in &s.def.abilities {
                let AbilityDef::SimpleStaticAbility { effects, .. } = a else { continue };
                for e in effects {
                    if let Some(given) = given_ability(e) {
                        if self.static_gives(e, *src, s.controller, id, given) {
                            add(given, &mut out);
                        }
                    }
                }
            }
        }
        for given in crate::rules::cards::abilities(self, id, lost_at) {
            add(&given, &mut out);
        }
        out
    }

    pub fn has_ability(&self, id: ObjId, ability: &AbilityDef) -> bool {
        let o = self.obj(id);
        if !self.lost_until_eot.is_empty()
            && self.lost_until_eot.iter().any(|(x, z, a, t)| *x == id && *z == o.zcc && a == ability && *t == self.turn)
        {
            return false;
        }
        // an Aura or Equipment's "enchanted creature loses …"
        if o.zone == Zone::Battlefield
            && self.any_static(|g, src, e| {
                matches!(e, EffectDef::LoseAbilityAttachedEffect { ability: lost } if **lost == *ability)
                    && g.attached(src) == Some(id)
            })
        {
            return false;
        }
        if o.has(ability) {
            return true;
        }
        // an indestructible counter
        if *ability == AbilityDef::IndestructibleAbility && o.counter("indestructible") > 0 {
            return true;
        }
        if *ability == AbilityDef::DeathtouchAbility && o.counter("deathtouch") > 0 {
            return true;
        }
        if *ability == AbilityDef::FlyingAbility && o.counter("flying") > 0 {
            return true;
        }
        // the other keyword counters (122.1b)
        let keyword = match ability {
            AbilityDef::LifelinkAbility => "lifelink",
            AbilityDef::TrampleAbility => "trample",
            AbilityDef::FirstStrikeAbility => "first strike",
            AbilityDef::DoubleStrikeAbility => "double strike",
            AbilityDef::MenaceAbility => "menace",
            AbilityDef::VigilanceAbility => "vigilance",
            AbilityDef::ReachAbility => "reach",
            AbilityDef::HexproofAbility => "hexproof",
            AbilityDef::HasteAbility => "haste",
            _ => "",
        };
        if !keyword.is_empty() && o.counter(keyword) > 0 {
            return true;
        }
        if o.zone != Zone::Battlefield {
            return false;
        }
        if self.grants.iter().any(|g| g.target == id && g.zcc == o.zcc && g.ability == *ability) {
            return true;
        }
        let lost_at = self.abilities_lost_at(id);
        self.battlefield.iter().chain(self.command.iter()).any(|src| {
            let s = self.obj(*src);
            !lost_at.is_some_and(|at| self.static_at(*src) < at)
                && s.def.abilities.iter().any(|a| match a {
                    AbilityDef::SimpleStaticAbility { effects, .. } => {
                        effects.iter().any(|e| self.static_gives(e, *src, s.controller, id, ability))
                    }
                    _ => false,
                })
        }) || crate::rules::cards::abilities(self, id, lost_at).contains(ability)
    }

    /// The timestamp of the newest attached Aura's "enchanted creature loses all abilities and is a …"
    /// (`BecomesCreatureAttachedEffect`): layer 6 abilities older static abilities give it are lost with the rest
    pub fn abilities_lost_at(&self, id: ObjId) -> Option<u64> {
        let o = self.obj(id);
        if o.becomes_auras.is_empty() {
            return None;
        }
        o.becomes_auras
            .iter()
            .filter(|a| self.attached(**a) == Some(id))
            .filter(|a| {
                self.obj(**a).def.abilities.iter().any(|b| match b {
                    AbilityDef::SimpleStaticAbility { effects, .. } => effects.iter().any(|e| {
                        matches!(e, EffectDef::BecomesCreatureAttachedEffect { lose: Some(l), .. }
                            if matches!(l.as_str(), "ALL" | "ALL_BUT_COLOR" | "ABILITIES" | "ABILITIES_SUBTYPE"))
                    }),
                    _ => false,
                })
            })
            .map(|a| self.obj(*a).attached_at)
            .max()
    }

    /// A permanent's static abilities' timestamp: when it was attached, for an attachment, else when it entered
    pub fn static_at(&self, src: ObjId) -> u64 {
        let s = self.obj(src);
        if self.attached(src).is_some() {
            s.attached_at
        } else {
            s.entered_at
        }
    }

    /// Whether a condition holds for `you`, the controller of `source`. `Kicked` holds only where the kicked spell is
    /// known (`rules::condition_with_kicker`).
    /// The triggered abilities of `sources` that trigger on an event (`fires`: for a source, its controller and its
    /// trigger) and whose intervening "if" holds, in Java's registration order.
    pub fn triggers(
        &self,
        sources: impl IntoIterator<Item = ObjId>,
        mut fires: impl FnMut(&Game, ObjId, Seat, &Trigger) -> bool,
    ) -> Vec<StackItem> {
        let mut fired = Vec::new();
        for src in sources {
            let o = self.obj(src);
            // one that left in the same action: the abilities it had
            let lki = o.lki_def.as_ref().filter(|_| o.zone != Zone::Battlefield && self.lookback.contains(&src));
            for (i, a) in lki.unwrap_or(&o.def).abilities.iter().enumerate() {
                if let Some(t) = a.trigger() {
                    let hit = match t {
                        Trigger::Or { ts } => ts.iter().any(|x| fires(self, src, o.controller, x)),
                        _ => fires(self, src, o.controller, t),
                    };
                    if hit
                        && self.condition_holds(a.intervening_if(), o.controller, src)
                        && self.condition_holds(a.trigger_condition(), o.controller, src)
                    {
                        // a copy until end of turn (Shifting Woodland) keeps what it copied once it has left; a Room's
                        // keeps the face it came from (the face is rebuilt as doors unlock)
                        let room = o.printed_def.as_ref().is_some_and(|f| !f.room.is_empty());
                        // (and one an effect gave it: the ability stays when the effect ends, Infernal Scarring's
                        // "When this creature dies, draw a card." with the Aura gone, Antagonist Incorporator's with
                        // it gone)
                        // (a transformed one's face: Wayward Disciple dying, its card Pious Evangel again)
                        let def = (o.until_end_of_turn_def.is_some()
                            || room
                            || o.land_type_base.is_some()
                            || (o.front.is_some() && o.zone == Zone::Battlefield))
                            .then(|| o.def.clone())
                            .or_else(|| lki.cloned());
                        fired.push(StackItem { ability: Some((i, o.zcc)), def, ..StackItem::spell(src, o.controller) });
                    }
                }
            }
        }
        let mut fired = self.additional_triggers(fired);
        fired.sort_by_key(|t| self.trigger_key(t));
        fired
    }

    /// `NumberOfTriggersEvent`: each trigger once more for each `AdditionalTriggersEffect` that applies to it, the
    /// copies next to it; with two or more applying, its controller is asked their order (the answers change nothing)
    fn additional_triggers(&self, fired: Vec<StackItem>) -> Vec<StackItem> {
        if fired.is_empty()
            || !(self.any_static(|_, _, e| matches!(e, EffectDef::AdditionalTriggersEffect { .. }))
                || crate::rules::cards::any_replacement(self, crate::rules::cards::EventType::NumberOfTriggers))
        {
            return fired;
        }
        let mut out = Vec::with_capacity(fired.len());
        for t in fired {
            let mut n = 0;
            self.any_static(|g, src, e| {
                if let EffectDef::AdditionalTriggersEffect { source, entered, died, condition, on_battlefield } = e {
                    let you = g.obj(src).controller;
                    let caused = |f: &Option<Filter>, dying: bool| match f {
                        None => true,
                        Some(f) => match g.trigger_cause {
                            Some(TriggerCause::Entered(id)) if !dying => g.filter_matches(f, you, Some(src), id),
                            Some(TriggerCause::Died(id)) if dying => g.filter_matches(f, you, Some(src), id),
                            _ => false,
                        },
                    };
                    if t.controller == you
                        && (!*on_battlefield || g.obj(t.card).zone == Zone::Battlefield)
                        && source.as_ref().is_none_or(|f| g.filter_matches(f, you, Some(src), t.card))
                        && caused(entered, false)
                        && caused(died, true)
                        && g.condition_holds(condition.as_ref(), you, src)
                    {
                        n += 1;
                    }
                }
                false
            });
            // static handlers' (`card_statics.rs`): each adds one
            n += crate::rules::cards::extra_triggers(self, &t);
            for _ in 1..n {
                self.owed_replacement_choices.borrow_mut().push(t.controller);
            }
            for _ in 0..n {
                out.push(t.clone());
            }
            out.push(t);
        }
        out
    }

    /// Pelt Collector's triggers on `id` entering or dying: each remembers `id`'s power, and fires only if it's
    /// greater than its source's
    pub fn pelt_collector(&self, mut fired: Vec<StackItem>, id: ObjId) -> Vec<StackItem> {
        let power = self.power_toughness(id).0;
        for t in fired.iter_mut() {
            if self.trigger_of(t) == Some(&Trigger::PeltCollector) {
                t.lki_power = Some(power);
            }
        }
        fired.retain(|t| self.pelt_holds(t));
        fired
    }

    pub fn trigger_of<'a>(&'a self, t: &'a StackItem) -> Option<&'a Trigger> {
        t.ability.and_then(|(i, _)| self.item_def(t).abilities.get(i)).and_then(|a| a.trigger())
    }

    /// `DISCARDED_CARDS`, once for the cards each player discarded together: "whenever you discard one or more
    /// cards", X the number of them of the filter
    pub fn discarded_cards(&mut self) {
        let batch = std::mem::take(&mut self.discarded);
        let mut seats: Vec<Seat> = Vec::new();
        for (s, _) in &batch {
            if !seats.contains(s) {
                seats.push(*s);
            }
        }
        for seat in seats {
            let cards: Vec<ObjId> = batch.iter().filter(|(s, _)| *s == seat).map(|(_, c)| *c).collect();
            let sources = self.trigger_sources();
            let fired = self.triggers(sources, |g, src, you, t| match t {
                Trigger::DiscardsOneOrMore { filter } => {
                    you == seat && cards.iter().any(|c| g.filter_matches(filter, you, Some(src), *c))
                }
                _ => false,
            });
            for mut t in fired {
                if let Some(Trigger::DiscardsOneOrMore { filter }) = self.trigger_of(&t) {
                    let you = t.controller;
                    t.x = cards.iter().filter(|c| self.filter_matches(filter, you, Some(t.card), **c)).count() as i32;
                }
                self.pending.push(t);
            }
        }
    }

    /// The card face a spell or ability comes from (an ability keeps the face it came from)
    pub fn item_def<'a>(&'a self, t: &'a StackItem) -> &'a Arc<CardDef> {
        t.def.as_ref().unwrap_or(&self.obj(t.card).def)
    }

    /// Pelt Collector's intervening "if": the creature's power is greater than its source's (still on the
    /// battlefield)
    /// A card's own trigger class's check (`src/cards/`, `Handler::trigger`), on a waiting or resolving trigger
    /// whose event details (`entering`, its target pointer) are set
    pub fn trigger_check_holds(&self, t: &StackItem) -> bool {
        match t.ability.and_then(|(i, _)| self.item_def(t).abilities.get(i)) {
            Some(AbilityDef::TriggeredAbility { check: Some(h), .. }) => h.f()(self, t),
            _ => true,
        }
    }

    pub fn pelt_holds(&self, t: &StackItem) -> bool {
        if self.trigger_of(t) != Some(&Trigger::PeltCollector) {
            return true;
        }
        let o = self.obj(t.card);
        let here = o.zone == Zone::Battlefield && t.ability.is_some_and(|(_, zcc)| zcc == o.zcc);
        here && t.lki_power.unwrap_or(0) > self.power_toughness(t.card).0
    }

    /// `who` ("you", "opponent", "any") of a trigger, for its controller `you` and the player `seat` it's about.
    /// The permanent an Aura is attached to, while it's still that object on the battlefield.
    /// `Permanent.addAttachment`: attached, after what's attached already
    /// The next timestamp (Java's `ContinuousEffects.setOrder`)
    pub fn timestamp(&mut self) -> u64 {
        self.attach_clock += 1;
        self.attach_clock
    }

    pub fn attach_to(&mut self, id: ObjId, to: ObjId, zcc: u32) {
        let at = self.timestamp();
        let key = (1 << 40) + self.objs.len() as u64;
        let o = self.obj_mut(id);
        o.attached_to = Some((to, zcc));
        o.attached_at = at;
        o.attached_key = key;
        if self.events.is_some() {
            let player = Some(self.obj(id).controller);
            self.record(GameEvent {
                source: Some(id),
                target: Some(Target::Obj(to)),
                player,
                ..GameEvent::new(EventKind::Attached)
            });
        }
    }

    /// the permanents attached to `id`
    pub fn attachments(&self, id: ObjId) -> impl Iterator<Item = ObjId> + '_ {
        self.battlefield.iter().copied().filter(move |a| self.attached(*a) == Some(id))
    }

    pub fn attached(&self, aura: ObjId) -> Option<ObjId> {
        let (to, zcc) = self.obj(aura).attached_to?;
        let o = self.obj(to);
        (o.zone == Zone::Battlefield && o.zcc == zcc).then_some(to)
    }

    /// Where a waiting trigger is in Java's trigger order: its source's, or for an ability a Class level added, after
    /// every object's own (in the order the levels were gained)
    pub fn trigger_key(&self, t: &StackItem) -> u64 {
        let o = self.obj(t.card);
        if let Some((i, _)) = t.ability {
            if let Some(crate::card::Condition::ClassLevel { n }) =
                o.def.abilities.get(i).and_then(|a| a.trigger_condition())
            {
                if let Some((_, k)) = o.class_gained.iter().find(|(l, _)| l == n) {
                    return *k;
                }
            }
            // a triggered ability an Aura or Equipment gives it (`GainAbilityAttachedEffect`): registered as the effect
            // applies, after the objects there were as it was attached (Hunter's Axe's "whenever this creature
            // attacks" after Hunter's Talent's level 2 "whenever you attack" from before)
            if let Some(a) = o.def.abilities.get(i) {
                let given = self
                    .attachments(t.card)
                    .filter(|s| {
                        self.obj(*s).def.abilities.iter().any(|x| {
                            matches!(x, crate::card::AbilityDef::SimpleStaticAbility { effects, .. }
                                if effects.iter().any(|e| matches!(e, crate::card::EffectDef::GainAbilityAttachedEffect { ability } if **ability == *a)))
                        })
                    })
                    .map(|s| self.obj(s).attached_key)
                    .min();
                if let Some(k) =
                    given.filter(|_| o.land_type_base.as_ref().is_some_and(|(b, _, _)| !b.abilities.contains(a)))
                {
                    return k;
                }
            }
            // a triggered ability an effect gave it (`GainAbilityTargetEffect`): Java registers it as the effect
            // applies, after every object's own (Fake Your Own Death's dies trigger after Vat of Rebirth's)
            if let (Some((base, _, gave)), Some(a)) = (&o.land_type_base, o.def.abilities.get(i)) {
                if gave.contains(a) && !base.abilities.contains(a) {
                    if let Some(k) = self.grants.iter().position(|g| g.target == t.card && g.ability == *a) {
                        return (1 << 40) + k as u64;
                    }
                }
            }
        }
        // a transformed permanent's abilities are its back face's, added again under new ids each time the game
        // resets its permanents, so they wait after every other card's, in battlefield order (`PermanentCard.reset`)
        if o.front.is_some() && !o.token {
            if let Some(i) = self.battlefield.iter().position(|b| *b == t.card) {
                return (3 << 32) + i as u64;
            }
        }
        o.trigger_order
    }

    /// Whether a static ability's effect on the battlefield matches (for effects that apply to the whole game).
    /// Keeps a game event, while they're kept
    /// `seat`'s pool payment: (manual, unrestricted); see `pool_modes`
    pub fn pool_mode(&self, seat: Seat) -> (bool, bool) {
        let bits = self.pool_modes.load(std::sync::atomic::Ordering::Relaxed) >> (2 * seat);
        (bits & 1 != 0, bits & 2 != 0)
    }

    pub fn set_pool_mode(&self, seat: Seat, manual: bool, unrestricted: bool) {
        use std::sync::atomic::Ordering;
        let mask = 3u32 << (2 * seat);
        let bits = (u32::from(manual) | (u32::from(unrestricted) << 1)) << (2 * seat);
        // the seat's two bits: cleared, then set (only the host's thread writes them)
        self.pool_modes.fetch_and(!mask, Ordering::Relaxed);
        self.pool_modes.fetch_or(bits, Ordering::Relaxed);
    }

    pub fn record(&mut self, mut event: GameEvent) {
        event.turn = self.turn;
        if let Some(events) = self.events.as_mut() {
            events.push(event);
        }
    }

    /// The printing a card was put in its deck as (`SET:number`, optionally `:imageNumber`): its views show it
    pub fn set_printing(&mut self, id: ObjId, printing: &str) {
        let all = Arc::make_mut(&mut self.printings);
        if all.len() <= id.0 as usize {
            all.resize(id.0 as usize + 1, String::new());
        }
        all[id.0 as usize] = printing.to_string();
    }

    /// The printing set with `set_printing`: its set code, collector number and image number, if any
    pub fn printing(&self, id: ObjId) -> Option<(&str, &str, Option<u32>)> {
        let p = self.printings.get(id.0 as usize).filter(|p| !p.is_empty())?;
        let mut parts = p.splitn(3, ':');
        let set = parts.next()?;
        let number = parts.next()?;
        Some((set, number, parts.next().and_then(|n| n.parse().ok())))
    }

    /// Whether `seat` may play `id` now by an effect that made it playable (`may_play`, with its end)
    pub fn may_play_now(&self, id: ObjId, seat: Seat) -> bool {
        let o = self.obj(id);
        o.may_play.is_some_and(|(p, t)| {
            p == seat
                && (self.turn < t
                    || (self.turn == t
                        && !(o.may_play_end_step && matches!(self.step, Some(Step::EndTurn | Step::Cleanup)))))
        }) && o.may_play_while.is_none_or(|(src, zcc)| {
            let s = self.obj(src);
            s.zone == Zone::Battlefield && s.zcc == zcc && s.controller == seat
        })
    }

    pub fn any_static(&self, mut f: impl FnMut(&Game, ObjId, &EffectDef) -> bool) -> bool {
        self.battlefield.iter().any(|src| {
            self.obj(*src).def.abilities.iter().any(|a| match a {
                AbilityDef::SimpleStaticAbility { effects, .. } => effects.iter().any(|e| f(self, *src, e)),
                _ => false,
            })
        }) || self.grants.iter().any(|g| match &g.ability {
            // a static ability a resolved effect gave a permanent (backup's)
            AbilityDef::SimpleStaticAbility { effects, .. } => {
                let o = self.obj(g.target);
                o.zone == Zone::Battlefield && o.zcc == g.zcc && effects.iter().any(|e| f(self, g.target, e))
            }
            _ => false,
        })
    }

    /// Whether a static ability of an attached Aura says `id` can't attack or block.
    /// Lovestruck Beast's restriction: it can't attack while its controller has no 1/1 creature
    pub fn cant_attack_alone(&self, id: ObjId) -> bool {
        let you = self.obj(id).controller;
        self.any_static(|g, src, e| {
            src == id
                && matches!(e, EffectDef::LovestruckBeastEffect)
                && !g.battlefield.iter().any(|c| {
                    let o = g.obj(*c);
                    o.controller == you && o.is(CardType::Creature) && g.power_toughness(*c) == (1, 1)
                })
        })
    }

    /// `checkAttackRequirements`: it attacks each combat if able
    pub fn must_attack(&self, id: ObjId) -> bool {
        let o = self.obj(id);
        o.def.abilities.iter().any(|a| match a {
            AbilityDef::SimpleStaticAbility { effects, .. } => effects.iter().any(|e| match e {
                EffectDef::AttacksIfAbleSourceEffect { condition } => {
                    self.condition_holds(condition.as_ref(), o.controller, id)
                }
                _ => false,
            }),
            _ => false,
        }) || self.any_static(|g, src, e| match e {
            EffectDef::AttacksIfAbleAllEffect { filter } => {
                g.filter_matches(filter, g.obj(src).controller, Some(src), id)
            }
            // one a resolved effect gave it (Legion Warboss's token)
            EffectDef::AttacksIfAbleSourceEffect { condition } => {
                src == id && g.condition_holds(condition.as_ref(), g.obj(src).controller, src)
            }
            EffectDef::AttacksIfAbleAttachedEffect => g.attached(src) == Some(id),
            _ => false,
        }) || self.requirements.iter().any(|r| r.kind == RequirementKind::Attacks && r.obj == (id, self.obj(id).zcc))
    }

    pub fn cant_attack_or_block(&self, id: ObjId) -> bool {
        let o = self.obj(id);
        if o.def.abilities.iter().any(|a| match a {
            AbilityDef::SimpleStaticAbility { effects, .. } => effects.iter().any(|e| match e {
                EffectDef::CantAttackBlockUnlessConditionSourceEffect { condition } => {
                    !self.condition_holds(Some(condition), o.controller, id)
                }
                _ => false,
            }),
            _ => false,
        }) {
            return true;
        }
        self.battlefield.iter().any(|src| {
            self.attached(*src) == Some(id)
                && self.obj(*src).def.abilities.iter().any(|a| match a {
                    AbilityDef::SimpleStaticAbility { effects, .. } => effects.iter().any(|e| {
                        matches!(
                            e,
                            EffectDef::CantAttackBlockAttachedEffect | EffectDef::CantBlockAttackActivateAttachedEffect
                        )
                    }),
                    _ => false,
                })
        })
    }

    /// What triggered abilities can come from: permanents, then emblems.
    /// the card cast with flashback: its own, or the one it gained this turn for its mana cost
    pub fn flashback_def(&self, id: ObjId) -> Option<CardDef> {
        let o = self.obj(id);
        o.def.flashback().or_else(|| {
            (o.flashback_turn == Some(self.turn))
                .then(|| CardDef { printed_cost: Some(o.def.printed().clone()), ..(*o.def).clone() })
        })
    }

    /// The stack's and the waiting triggers' abilities of `id` moved to their new index in its list (`map[old]`); one
    /// that's gone keeps its index
    fn remap_abilities(&mut self, id: ObjId, map: &[Option<usize>]) {
        let moved = |i: usize| map.get(i).copied().flatten().unwrap_or(i);
        let on_stack = self.stack.iter().any(|t| t.card == id && t.ability.is_some());
        if on_stack {
            for t in Arc::make_mut(&mut self.stack).iter_mut() {
                if let (true, Some((i, z))) = (t.card == id, t.ability) {
                    t.ability = Some((moved(i), z));
                }
            }
        }
        for t in self.pending.iter_mut() {
            if let (true, Some((i, z))) = (t.card == id, t.ability) {
                t.ability = Some((moved(i), z));
            }
        }
        for (o, _, i) in self.triggered_this_turn.iter_mut() {
            if *o == id {
                *i = moved(*i);
            }
        }
        for (o, i) in self.activated_this_turn.iter_mut() {
            if *o == id {
                *i = moved(*i);
            }
        }
    }

    /// whether a Leyline of the Guildpact (or the like) is on the battlefield under `seat`'s control, with this effect
    pub fn guildpact(&self, seat: Seat, land_part: bool) -> bool {
        if land_part && self.all_basics_until_eot.contains(&seat) {
            return true;
        }
        self.battlefield.iter().any(|src| {
            let s = self.obj(*src);
            s.controller == seat
                && s.def.abilities.iter().any(|a| match a {
                    AbilityDef::SimpleStaticAbility { effects, .. } => effects.iter().any(|e| {
                        if land_part {
                            matches!(e, EffectDef::BecomesAllBasicsControlledEffect)
                        } else {
                            matches!(e, EffectDef::LeylineOfTheGuildpactEffect)
                        }
                    }),
                    _ => false,
                })
        })
    }

    /// An object's colors: all five for a nonland permanent under Leyline of the Guildpact
    pub fn colors(&self, id: ObjId) -> Vec<crate::card::Color> {
        let o = self.obj(id);
        if o.zone == Zone::Battlefield && !o.is(CardType::Land) && self.guildpact(o.controller, false) {
            return crate::card::Color::ALL.to_vec();
        }
        o.def.colors.clone()
    }

    /// `BecomesAllBasicsControlledEffect`: the lands of a player with it are every basic land type, their basic mana
    /// abilities replaced by one of any color; undone when it's gone. The any-color one comes after the land's own
    /// abilities and before what layer 6 gave it (an Aura's granted ones, refresh_land_types); basic land types
    /// another static gave (Urborg) keep their mana ability, after the any-color one, when that static came later than
    /// the Leyline (Java applies layer 4 by timestamp: the Leyline's removal ran before Urborg added its ability)
    pub fn update_guildpact(&mut self) {
        let lands: Vec<ObjId> = self.battlefield.iter().copied().filter(|l| self.obj(*l).is(CardType::Land)).collect();
        for l in lands {
            let on = self.guildpact(self.obj(l).controller, true);
            match (on, self.obj(l).pre_guildpact.is_some()) {
                (true, false) => self.apply_guildpact(l),
                (false, true) => self.unapply_guildpact(l),
                _ => {}
            }
        }
    }

    /// `AscendContinuousEffect` (layer PlayerEffects): the controller of a permanent with ascend who controls ten or
    /// more permanents gets the city's blessing
    pub fn update_ascend(&mut self) {
        for s in 0..self.seats() {
            if self.player(s).citys_blessing {
                continue;
            }
            let mine: Vec<ObjId> = self.battlefield.iter().copied().filter(|o| self.obj(*o).controller == s).collect();
            if mine.len() > 9 && mine.iter().any(|o| self.has_ability(*o, &AbilityDef::AscendAbility)) {
                self.player_mut(s).citys_blessing = true;
            }
        }
    }

    /// The Leyline whose land part applies to `seat`'s lands, by its place on the battlefield (its timestamp)
    fn guildpact_at(&self, seat: Seat) -> Option<usize> {
        self.battlefield.iter().position(|src| {
            let s = self.obj(*src);
            s.controller == seat
                && s.def.abilities.iter().any(|a| {
                    matches!(a, AbilityDef::SimpleStaticAbility { effects, .. }
                        if effects.contains(&EffectDef::BecomesAllBasicsControlledEffect))
                })
        })
    }

    fn apply_guildpact(&mut self, l: ObjId) {
        let o = self.obj(l);
        let own = o.def.clone();
        let len = own.abilities.len();
        // what refresh_land_types added at the end: the given types' mana abilities, then layer 6's
        let (given_n, gained_n) = match &o.land_type_base {
            Some((base, _, gained)) => {
                let gained_n = gained.len().min(len);
                (len.saturating_sub(gained_n).saturating_sub(base.abilities.len()), gained_n)
            }
            None => (0, 0),
        };
        let leyline = self.guildpact_at(o.controller).unwrap_or(0);
        let later_types = self.battlefield.iter().position(|src| {
            self.obj(*src).def.abilities.iter().any(|a| {
                matches!(a, AbilityDef::SimpleStaticAbility { effects, .. }
                    if effects.iter().any(|e| matches!(e, EffectDef::AddBasicLandTypeAllLandsEffect { .. })))
            })
        });
        let keep_given = later_types.is_some_and(|t| t > leyline);
        let head_end = len - gained_n - given_n;
        let any = AbilityDef::AnyColorManaAbility {
            amount: 1,
            effects: Vec::new(),
            cost: Vec::new(),
            no_tap: false,
            rule: "{T}: Add one mana of any color.".into(),
        };
        let has_any = own.abilities.iter().any(|a| {
            matches!(a, AbilityDef::AnyColorManaAbility { amount: 1, effects, cost, .. }
                if effects.is_empty() && cost.is_empty())
        });
        // the new list, each by its old index (None: the any-color one)
        let mut order: Vec<Option<usize>> =
            (0..head_end).filter(|i| !guildpact_basic(&own.abilities[*i])).map(Some).collect();
        if !has_any {
            order.push(None);
        }
        order.extend(
            (head_end..head_end + given_n).filter(|i| keep_given || !guildpact_basic(&own.abilities[*i])).map(Some),
        );
        order.extend((head_end + given_n..len).map(Some));
        let map: Vec<Option<usize>> = (0..len).map(|i| order.iter().position(|k| *k == Some(i))).collect();
        self.remap_abilities(l, &map);
        let mut def = (*own).clone();
        for t in ["Plains", "Island", "Swamp", "Mountain", "Forest"] {
            if !def.subtypes.iter().any(|s| s == t) {
                def.subtypes.push(t.to_string());
            }
        }
        def.abilities =
            order.iter().map(|k| k.map(|i| own.abilities[i].clone()).unwrap_or_else(|| any.clone())).collect();
        let o = self.obj_mut(l);
        o.guildpact_order = order;
        o.pre_guildpact = Some(std::mem::replace(&mut o.def, Arc::new(def)));
    }

    /// Takes Leyline of the Guildpact's types and mana ability off a land, back to what it was without them.
    pub fn unapply_guildpact(&mut self, l: ObjId) {
        let Some(own) = self.obj_mut(l).pre_guildpact.take() else { return };
        let order = std::mem::take(&mut self.obj_mut(l).guildpact_order);
        self.obj_mut(l).def = own;
        self.remap_abilities(l, &order);
    }

    /// every object whose `p1p1_added` is set
    pub fn objs_with_p1p1_added(&self) -> Vec<ObjId> {
        (0..self.objs.len()).map(|i| ObjId(i as u32)).filter(|o| self.obj(*o).p1p1_added).collect()
    }

    pub fn trigger_sources(&self) -> Vec<ObjId> {
        self.battlefield.iter().chain(self.command.iter()).copied().collect()
    }

    pub fn whose(who: &str, you: Seat, seat: Seat) -> bool {
        match who {
            "you" => you == seat,
            "opponent" => you != seat,
            _ => true,
        }
    }

    /// An effect's number for `you`, the controller of `source`, with `x` announced.
    pub fn value(&self, a: &Amount, you: Seat, source: ObjId, x: i32) -> i32 {
        match a {
            Amount::Fixed(n) => *n,
            Amount::Of(v) => match v {
                Value::Card { handler } => handler.f()(self, you, source, x),
                Value::X => x,
                Value::Permanents { filter } => {
                    self.battlefield.iter().filter(|id| self.filter_matches(filter, you, Some(source), **id)).count()
                        as i32
                }
                Value::CardsInHand => self.player(you).hand.len() as i32,
                Value::CardsDrawn => self.player(you).drawn as i32,
                Value::LifeGained => self.player(you).life_gained,
                Value::ManaExpended => self.player(you).expended as i32,
                Value::SacrificedThisTurn => self.player(you).sacrificed as i32,
                Value::SacrificedPower => self.resolving_item.as_ref().map_or(0, |i| i.sacrificed_stats[0]),
                Value::SacrificedToughness => self.resolving_item.as_ref().map_or(0, |i| i.sacrificed_stats[1]),
                Value::SacrificedManaValue => self.resolving_item.as_ref().map_or(0, |i| i.sacrificed_stats[2]),
                Value::OpponentsLostLife => {
                    (0..self.seats()).filter(|p| *p != you && self.player(*p).life_lost > 0).count() as i32
                }
                Value::CardsInGraveyard { filter, all } => (0..self.seats())
                    .filter(|s| *all || *s == you)
                    .flat_map(|s| self.player(s).graveyard.iter())
                    .filter(|id| self.filter_matches(filter, you, Some(source), **id))
                    .count() as i32,
                // a source gone from the battlefield: its power as it last was there
                Value::SourcePower => match self.obj(source).power_lki {
                    Some(p) if self.obj(source).zone != Zone::Battlefield => p,
                    _ => self.power_toughness(source).0,
                },
                Value::SourceToughness => self.power_toughness(source).1,
                Value::ControllerLife => self.player(you).life,
                Value::TotalPower { filter } => self
                    .battlefield
                    .iter()
                    .filter(|id| self.filter_matches(filter, you, Some(source), **id))
                    .map(|id| self.power_toughness(*id).0)
                    .sum(),
                Value::Negative { of } => -self.value(of, you, source, x),
                Value::Times { of, n } => self.value(of, you, source, x) * n,
                Value::Half { of } => self.value(of, you, source, x) / 2,
                Value::Plus { of, n } => self.value(of, you, source, x) + n,
                Value::ExiledWithSource => {
                    let zcc = self.obj(source).zcc;
                    self.exiled_for_source.iter().filter(|(s, z, _)| *s == source && *z == zcc).count() as i32
                }
                Value::CardTypesInYourGraveyard => {
                    let mut types: Vec<CardType> = Vec::new();
                    for c in self.player(you).graveyard.iter() {
                        for t in &self.obj(*c).def.types {
                            if !types.contains(t) {
                                types.push(*t);
                            }
                        }
                    }
                    types.len() as i32
                }
                Value::CardTypesInGraveyards => {
                    let mut types: Vec<CardType> = Vec::new();
                    for p in self.players.iter() {
                        for c in p.graveyard.iter() {
                            for t in &self.obj(*c).def.types {
                                if !types.contains(t) {
                                    types.push(*t);
                                }
                            }
                        }
                    }
                    types.len() as i32
                }
                Value::CardTypesInOpponentsGraveyards => {
                    let mut types: Vec<CardType> = Vec::new();
                    for (seat, p) in self.players.iter().enumerate() {
                        if seat == you {
                            continue;
                        }
                        for c in p.graveyard.iter() {
                            for t in &self.obj(*c).def.types {
                                if !types.contains(t) {
                                    types.push(*t);
                                }
                            }
                        }
                    }
                    types.len() as i32
                }
                Value::GreatestManaValue { filter } => self
                    .battlefield
                    .iter()
                    .filter(|b| self.filter_matches(filter, you, Some(source), **b))
                    .map(|b| self.obj(*b).def.mana_value() as i32)
                    .max()
                    .unwrap_or(0),
                Value::GreatestPower { filter } => self
                    .battlefield
                    .iter()
                    .filter(|b| self.filter_matches(filter, you, Some(source), **b))
                    .map(|b| self.power_toughness(*b).0)
                    .max()
                    .unwrap_or(0),
                Value::SourceCounters { counter } => {
                    let o = self.obj(source);
                    if o.zone == Zone::Battlefield {
                        o.counter(counter)
                    } else {
                        // `CountersSourceCount` looks back at it (Dragonspark Reactor sacrificed for its damage)
                        o.lki_counters.iter().filter(|(c, _)| c == counter).map(|c| c.1).sum()
                    }
                }
                Value::AurasAttached => self
                    .battlefield
                    .iter()
                    .filter(|a| self.attached(**a) == Some(source) && self.has_subtype(**a, "Aura"))
                    .count() as i32,
                Value::Domain => ["Plains", "Island", "Swamp", "Mountain", "Forest"]
                    .iter()
                    .filter(|t| {
                        self.battlefield.iter().any(|b| {
                            let o = self.obj(*b);
                            o.controller == you && o.is(CardType::Land) && self.has_subtype(*b, t)
                        })
                    })
                    .count() as i32,
                Value::CreaturesDied => self.creatures_died as i32,
                Value::EquipmentAttached => {
                    self.attachments(source).filter(|a| self.has_subtype(*a, "Equipment")).count() as i32
                }
                Value::NonlandsEntered => self.player(you).nonlands_entered as i32,
                Value::Descended => self.player(you).descended as i32,
                Value::DifferentPowers { filter } => {
                    let mut powers: Vec<i32> = Vec::new();
                    for b in self.battlefield.iter() {
                        if self.filter_matches(filter, you, Some(source), *b) {
                            let p = self.power_toughness(*b).0;
                            if !powers.contains(&p) {
                                powers.push(p);
                            }
                        }
                    }
                    powers.len() as i32
                }
                Value::Party => {
                    let mine: Vec<ObjId> = self
                        .battlefield
                        .iter()
                        .copied()
                        .filter(|b| self.obj(*b).controller == you && self.has_type(*b, CardType::Creature))
                        .collect();
                    self.party_size(&mine)
                }
                Value::ColorsAmong { filter } => {
                    let mut colors: Vec<crate::card::Color> = Vec::new();
                    for b in self.battlefield.iter() {
                        if self.filter_matches(filter, you, Some(source), *b) {
                            for c in self.colors(*b) {
                                if !colors.contains(&c) {
                                    colors.push(c);
                                }
                            }
                        }
                    }
                    colors.len() as i32
                }
                Value::ColorsSpent => self.obj(source).spent_colors.iter().filter(|n| **n > 0).count() as i32,
                Value::SourceCountersAny => {
                    let o = self.obj(source);
                    if o.zone == Zone::Battlefield {
                        o.counters.iter().map(|(_, n)| *n).sum()
                    } else {
                        o.lki_counters.iter().map(|(_, n)| *n).sum()
                    }
                }
                Value::Sum { xs } => xs.iter().map(|a| self.value(a, you, source, x)).sum(),
                Value::Devotion { c, c2 } => self
                    .battlefield
                    .iter()
                    // a transformed permanent's back face has no mana symbols (its mana value is still its front's)
                    .filter(|b| {
                        let o = self.obj(**b);
                        o.controller == you && o.front.as_ref().is_none_or(|f| f.transform_back.is_none())
                    })
                    .map(|b| {
                        let cost = &self.obj(*b).def.cost;
                        (match c2 {
                            Some(c2) => cost.symbols_of_either(*c, *c2),
                            None => cost.symbols_of(*c),
                        }) as i32
                    })
                    .sum(),
            },
        }
    }

    /// The most of Cleric, Rogue, Warrior and Wizard these creatures can fill, each creature filling one
    /// (`PartyCount`'s `RoleAssignment`)
    pub fn party_size(&self, creatures: &[ObjId]) -> i32 {
        const ROLES: [&str; 4] = ["Cleric", "Rogue", "Warrior", "Wizard"];
        let fits: Vec<Vec<usize>> = creatures
            .iter()
            .map(|c| (0..ROLES.len()).filter(|r| self.has_subtype(*c, ROLES[*r])).collect())
            .filter(|r: &Vec<usize>| !r.is_empty())
            .collect();
        // the largest set of roles some creatures can fill one each: try every subset of roles, biggest first
        fn fill(fits: &[Vec<usize>], roles: &[usize], used: &mut Vec<bool>) -> bool {
            let Some((&r, rest)) = roles.split_first() else { return true };
            for (i, f) in fits.iter().enumerate() {
                if !used[i] && f.contains(&r) {
                    used[i] = true;
                    if fill(fits, rest, used) {
                        return true;
                    }
                    used[i] = false;
                }
            }
            false
        }
        let mut best = 0;
        for mask in 0u32..16 {
            let roles: Vec<usize> = (0..4).filter(|r| mask & (1 << r) != 0).collect();
            if roles.len() as i32 > best && fill(&fits, &roles, &mut vec![false; fits.len()]) {
                best = roles.len() as i32;
            }
        }
        best
    }

    pub fn condition_holds(&self, condition: Option<&Condition>, you: Seat, source: ObjId) -> bool {
        let Some(c) = condition else { return true };
        match c {
            Condition::Card { handler } => handler.f()(self, you, source),
            // a permanent that was cast kicked ("when this enters, if it was kicked")
            Condition::Kicked => {
                self.obj(source).kicked
                    || self.stack.iter().any(|i| i.card == source && i.ability.is_none() && i.kicked)
            }
            // only known on a cast trigger (`condition_with_kicker`)
            Condition::Opus => false,
            Condition::Offspring => self.obj(source).offspring,
            Condition::Evoked => self.obj(source).evoked,
            Condition::Impending => self.obj(source).impending,
            Condition::ManaColorSpent { color, n } => {
                let o = self.obj(source);
                // (a spell resolving: what was spent to cast it, Unexplained Vision's adamant)
                (o.zone == Zone::Stack || o.cast || (o.zone != Zone::Battlefield && o.cast_lki))
                    && o.spent_colors[color.index()] >= *n
            }
            // `CastFromEverywhereSourceCondition`: the permanent, or as it last was on the battlefield
            Condition::CastFromEverywhere => {
                let o = self.obj(source);
                o.cast || (o.zone != Zone::Battlefield && o.cast_lki)
            }
            Condition::Raid => self.player(you).attacked,
            Condition::CitysBlessing => self.player(you).citys_blessing,
            // only a resolving spell's conditional effect knows (`ConditionalOneShotEffect`)
            Condition::GiftPromised => false,
            // the werewolves' "if no spells were cast last turn" (not on the first turn) and "if a player cast two
            // or more spells last turn"
            Condition::NoSpellsLastTurn => self.turn >= 2 && self.players.iter().all(|p| p.spells_cast_last_turn == 0),
            Condition::TwoSpellsLastTurn => self.players.iter().any(|p| p.spells_cast_last_turn >= 2),
            Condition::Revolt => self.player(you).permanent_left,
            Condition::InstantsSorceriesCast { n } => self.player(you).instants_sorceries_cast >= *n,
            // `getStackObject`: the spell is still on the stack as it resolves, and its targets are as they last were
            // the resolving spell or ability's targets (Little Bear's trigger), else its spell's on the stack
            Condition::SourceTargets { filter } => {
                let resolving = self.resolving_item.as_ref().filter(|i| i.card == source);
                let mut items: Box<dyn Iterator<Item = &StackItem>> = match resolving {
                    Some(i) => Box::new(std::iter::once(i)),
                    None => Box::new(self.stack.iter().filter(|i| i.card == source && i.ability.is_none())),
                };
                items.any(|i| {
                    i.targets.iter().flatten().any(|t| match t {
                        Target::Obj(o) => self.filter_matches(filter, you, Some(source), *o),
                        _ => false,
                    })
                })
            }
            Condition::SourceAttacking => self.obj(source).attacking.is_some(),
            Condition::SourceSaddled => {
                let turn = format!("saddled{}", self.turn);
                self.obj(source).marks.contains(&turn)
            }
            // `PackTacticsAbility.checkTrigger`: the attackers' power, as they are now
            Condition::AttackingPower { n } => {
                self.battlefield
                    .iter()
                    .filter(|b| self.obj(**b).attacking.is_some())
                    .map(|b| self.power_toughness(*b).0)
                    .sum::<i32>()
                    >= *n
            }
            Condition::SourceRenowned => self.obj(source).marks.iter().any(|m| m == "renowned"),
            Condition::SourceTapped => self.obj(source).tapped,
            Condition::SourceAttackedThisTurn => {
                let turn = format!("attacked{}", self.turn);
                self.obj(source).marks.contains(&turn)
            }
            Condition::SourceHasCounters => self.obj(source).counters.iter().any(|(_, n)| *n > 0),
            Condition::SourceAttached { equipment } => self.battlefield.iter().any(|b| {
                self.attached(*b) == Some(source)
                    && (self.has_type(*b, CardType::Enchantment) || *equipment && self.has_subtype(*b, "Equipment"))
            }),
            Condition::SourceAttackingAlone => {
                self.obj(source).attacking.is_some()
                    && self.battlefield.iter().filter(|b| self.obj(**b).attacking.is_some()).count() == 1
            }
            Condition::AttackedOrBlockedThisCombat => {
                let o = self.obj(source);
                o.attacking.is_some() || !o.blocking.is_empty() || o.blocks > 0
            }
            Condition::SacrificedThisTurn { n } => self.player(you).sacrificed as i32 >= *n,
            Condition::ClassLevel { n } => {
                let o = self.obj(source);
                (if o.zone == Zone::Battlefield { o.class_level } else { o.lki_class_level }) >= *n
            }
            Condition::CardsInGraveyard { filter, n } => {
                let g = &self.player(you).graveyard;
                g.iter().filter(|c| self.filter_matches(filter, you, Some(source), **c)).count() as i32 >= *n
            }
            Condition::OpponentControlsMore { filter } => {
                let count = |p: Seat| {
                    self.battlefield
                        .iter()
                        .filter(|b| {
                            self.obj(**b).controller == p && self.filter_matches(filter, you, Some(source), **b)
                        })
                        .count()
                };
                (0..self.seats()).any(|p| p != you && count(p) > count(you))
            }
            Condition::OpponentMoreLife => {
                (0..self.seats()).any(|p| p != you && self.player(p).life > self.player(you).life)
            }
            Condition::CardsInHandAtMost { n } => self.player(you).hand.len() <= *n,
            Condition::OpponentMoreCards => {
                (0..self.seats()).any(|p| p != you && self.player(p).hand.len() > self.player(you).hand.len())
            }
            Condition::OpponentsLostLife => (0..self.seats()).any(|p| p != you && self.player(p).life_lost > 0),
            Condition::MyTurn => self.active == Some(you),
            Condition::FirstCombatPhase => !self.combat_phase_done,
            Condition::Morbid => self.creature_died,
            Condition::PlayedLand => self.player(you).lands_played > 0,
            Condition::CardsInExile { n } => self.exile.len() >= *n,
            Condition::Delirium => {
                self.value(&Amount::Of(crate::card::Value::CardTypesInYourGraveyard), you, source, 0) >= 4
            }
            Condition::FlierCast => self.player(you).flier_cast,
            Condition::YouGainedLife => self.player(you).life_gained > 0,
            Condition::LifeGained { cmp, n } => cmp.holds(self.player(you).life_gained, *n),
            Condition::DrewCards { n } => self.player(you).drawn >= *n,
            Condition::ExiledCardTypes { n } => {
                let zcc = self.obj(source).zcc;
                let mut types: Vec<CardType> = Vec::new();
                for (_, _, c) in self
                    .exiled_for_source
                    .iter()
                    .filter(|(s, z, c)| *s == source && *z == zcc && self.obj(*c).zone == Zone::Exile)
                {
                    for t in &self.obj(*c).def.types {
                        if !types.contains(t) {
                            types.push(*t);
                        }
                    }
                }
                types.len() >= *n
            }
            Condition::Solved => self.obj(source).zone == Zone::Battlefield && self.obj(source).solved,
            Condition::SourceEnteredThisTurn => self.obj(source).entered_turn == self.turn,
            Condition::StartingTown => self.active == Some(you) && self.turn.div_ceil(self.seats() as u32) <= 3,
            Condition::LifeCompare { who, cmp, n } => (0..self.seats())
                .filter(|s| if who == "you" { *s == you } else { *s != you })
                .any(|s| cmp.holds(self.player(s).life, *n)),
            Condition::Permanents { filter, cmp, n, all } => {
                let count = self
                    .battlefield
                    .iter()
                    .filter(|b| {
                        (*all || self.obj(**b).controller == you) && self.filter_matches(filter, you, Some(source), **b)
                    })
                    .count() as i32;
                cmp.holds(count, *n)
            }
            Condition::SourceCounter { name, n } => {
                let o = self.obj(source);
                o.zone == Zone::Battlefield && o.counter(name) >= *n
            }
            Condition::Value { of, cmp, n } => cmp.holds(self.value(of, you, source, 0), *n),
            Condition::CastFrom { zone } => {
                let z = match zone.as_str() {
                    "graveyard" => Zone::Graveyard,
                    "exile" => Zone::Exile,
                    _ => Zone::Hand,
                };
                let o = self.obj(source);
                o.cast_from == Some(z)
                    && match o.zone {
                        Zone::Stack => true,
                        Zone::Battlefield => o.cast,
                        _ => o.cast_lki,
                    }
            }
            Condition::OpponentPoison { n } => (0..self.seats()).any(|p| {
                p != you
                    && self.player(p).counters.iter().filter(|(k, _)| k == "poison").map(|(_, c)| *c).sum::<i32>() >= *n
            }),
            Condition::OpponentGraveyard { n } => {
                (0..self.seats()).any(|p| p != you && self.player(p).graveyard.len() >= *n)
            }
            Condition::Void => self.void_this_turn,
            Condition::SourceEquipped => {
                self.obj(source).zone == Zone::Battlefield
                    && self.attachments(source).any(|a| self.has_subtype(a, "Equipment"))
            }
            Condition::SourceTappedIs { v } => {
                self.obj(source).zone == Zone::Battlefield && self.obj(source).tapped == *v
            }
            Condition::SourceMatches { filter } => self.filter_matches(filter, you, Some(source), source),
            Condition::AttachedMatches { filter } => {
                self.obj(source).zone == Zone::Battlefield
                    && self.attached(source).is_some_and(|a| self.filter_matches(filter, you, Some(source), a))
            }
            Condition::Not { x } => !self.condition_holds(Some(x), you, source),
            Condition::And { xs } => xs.iter().all(|x| self.condition_holds(Some(x), you, source)),
            Condition::Or { xs } => xs.iter().any(|x| self.condition_holds(Some(x), you, source)),
        }
    }

    /// Whether a static ability's effect, from `src`, gives `id` the ability.
    /// Whether a player has an ability from a static ability of theirs (`GainAbilityControllerEffect`)
    pub fn player_has(&self, seat: Seat, ability: &AbilityDef) -> bool {
        fn gives(g: &Game, e: &EffectDef, src: ObjId, you: Seat, ability: &AbilityDef) -> bool {
            match e {
                EffectDef::GainAbilityControllerEffect { ability: given } => **given == *ability,
                EffectDef::ConditionalContinuousEffect { effect, condition } => {
                    g.condition_holds(Some(condition), you, src) && gives(g, effect, src, you, ability)
                }
                _ => false,
            }
        }
        self.any_static(|g, src, e| g.obj(src).controller == seat && gives(g, e, src, seat, ability))
    }

    pub fn static_gives(&self, e: &EffectDef, src: ObjId, you: Seat, id: ObjId, ability: &AbilityDef) -> bool {
        match e {
            EffectDef::GainAbilityAllEffect { ability: given, filter, exclude_source, .. } => {
                **given == *ability
                    && !(*exclude_source && id == src)
                    && self.filter_matches(filter, you, Some(src), id)
            }
            EffectDef::GainAbilityControlledEffect { ability: given, filter, exclude_source, .. } => {
                **given == *ability
                    && !(*exclude_source && id == src)
                    && self.obj(id).controller == you
                    && self.filter_matches(filter, you, Some(src), id)
            }
            EffectDef::GainAbilitySourceEffect { ability: given, .. } => **given == *ability && id == src,
            EffectDef::GainAbilityAttachedEffect { ability: given } => {
                **given == *ability && self.attached(src) == Some(id)
            }
            EffectDef::ConditionalContinuousEffect { effect, condition } => {
                self.static_gives(effect, src, you, id, ability) && self.condition_holds(Some(condition), you, src)
            }
            _ => false,
        }
    }

    /// Power and toughness with continuous effects applied: static abilities of permanents, then resolved boosts
    /// (layer 7c; everything here only adds, so the order doesn't matter yet).
    pub fn power_toughness(&self, id: ObjId) -> (i32, i32) {
        self.power_toughness_layers(id, true)
    }

    /// Power and toughness before counters and boosts: the printed values with layers 7a and 7b applied (Java's
    /// `MageInt.getModifiedBaseValue`)
    pub fn base_power_toughness(&self, id: ObjId) -> (i32, i32) {
        self.power_toughness_layers(id, false)
    }

    fn power_toughness_layers(&self, id: ObjId, boosts: bool) -> (i32, i32) {
        let o = self.obj(id);
        let (mut p, mut t) = (o.def.power, o.def.toughness);
        if o.zone != Zone::Battlefield {
            return (p, t);
        }
        // layer 7a: characteristic-defining abilities (that an Aura's base power and toughness in layer 7b replace:
        // Collective Drone enchanted by Ensoul Artifact is 5/5)
        let ensouled = o.becomes_base.is_some()
            && self.battlefield.iter().any(|a| {
                self.attached(*a) == Some(id)
                    && self.obj(*a).def.abilities.iter().any(|b| {
                        matches!(b, AbilityDef::SimpleStaticAbility { effects, .. }
                            if effects.iter().any(|e| matches!(e, EffectDef::BecomesCreatureAttachedEffect { .. })))
                    })
            });
        for a in o.def.abilities.iter().filter(|_| !ensouled) {
            if let AbilityDef::SimpleStaticAbility { effects, .. } = a {
                for e in effects {
                    if let EffectDef::SetBasePowerSourceEffect { amount } = e {
                        p = self.value(amount, o.controller, id, 0);
                    }
                    if let EffectDef::SetBasePowerToughnessPlusOneSourceEffect { amount } = e {
                        p = self.value(amount, o.controller, id, 0);
                        t = p + 1;
                    }
                    if let EffectDef::SetBasePowerToughnessAmountSourceEffect { amount } = e {
                        p = self.value(amount, o.controller, id, 0);
                        t = p;
                    }
                    if let EffectDef::SetBasePowerToughnessSourceEffect { count } = e {
                        if count == "exiled_with_source" {
                            let n = self.exiled_for_source.iter().filter(|(s, z, _)| *s == id && *z == o.zcc).count()
                                as i32;
                            p = n;
                            t = n;
                        }
                        // Harmonious Grovestrider: the lands you control
                        let counted = match count.as_str() {
                            "creatures_you_control" => Some(CardType::Creature),
                            "lands_you_control" => Some(CardType::Land),
                            _ => None,
                        };
                        if let Some(ty) = counted {
                            let n = self
                                .battlefield
                                .iter()
                                .filter(|b| {
                                    let x = self.obj(**b);
                                    x.controller == o.controller && x.is(ty)
                                })
                                .count() as i32;
                            p = n;
                            t = n;
                        }
                    }
                }
            }
        }
        // layer 7b, in timestamp order: the last one sets them
        let mut set: Option<(u64, i32, i32)> = o.base_pt.map(|(bp, bt)| (o.base_pt_at, bp, bt));
        let mut later = |at: u64, bp: i32, bt: i32| {
            if set.is_none_or(|(s, _, _)| at >= s) {
                set = Some((at, bp, bt));
            }
        };
        for (x, z, bp, bt, at) in self.base_pt_until_eot.iter() {
            if *x == id && *z == o.zcc {
                later(*at, *bp, *bt);
            }
        }
        for src in self.battlefield.iter() {
            let s = self.obj(*src);
            for a in &s.def.abilities {
                if let AbilityDef::SimpleStaticAbility { effects, .. } = a {
                    for e in effects {
                        if let Some((at, bp, bt)) = self.static_base_pt(e, *src, s.controller, id) {
                            later(at, bp, bt);
                        }
                    }
                }
            }
        }
        // static handlers' (`card_statics.rs`), by their timestamps with the others
        for (at, bp, bt) in crate::rules::cards::set_pt(self, id) {
            later(at, bp, bt);
        }
        if let Some((_, bp, bt)) = set {
            (p, t) = (bp, bt);
        }
        if !boosts {
            return (p, t);
        }
        // +1/+1 counters change only a creature's power and toughness (a Vehicle's, while it's crewed)
        if self.has_type(id, CardType::Creature) {
            let counters = o.counter("+1/+1") - o.counter("-1/-1");
            p += counters;
            t += counters;
        }
        // hone counters: each Equipment's give the creature it equips +1/+0 apiece (`ApplyStatusEffect`, layer 7c)
        if self.has_type(id, CardType::Creature) {
            for e in self.battlefield.iter() {
                if self.attached(*e) == Some(id) && self.obj(*e).def.has_subtype("Equipment") {
                    p += self.obj(*e).counter("hone");
                }
            }
        }
        for src in self.battlefield.iter().chain(self.command.iter()) {
            let s = self.obj(*src);
            for a in &s.def.abilities {
                if let AbilityDef::SimpleStaticAbility { effects, .. } = a {
                    for e in effects {
                        let (dp, dt) = self.static_boost(e, *src, s.controller, id);
                        p += dp;
                        t += dt;
                    }
                }
            }
        }
        let (dp, dt) = crate::rules::cards::modify_pt(self, id);
        p += dp;
        t += dt;
        // (`BoostTargetEffect` changes only a creature: a Vehicle's boost counts once it's crewed)
        let creature = self.has_type(id, CardType::Creature);
        for b in self.boosts.iter() {
            if b.target == id && b.zcc == o.zcc && (creature || b.noncreature_too) {
                p += b.power;
                t += b.toughness;
            }
        }
        (p, t)
    }

    /// A static ability's layer 7b effect on `id`: its timestamp (its source's, or an Aura's as it was attached) and
    /// the base power and toughness
    fn static_base_pt(&self, e: &EffectDef, src: ObjId, you: Seat, id: ObjId) -> Option<(u64, i32, i32)> {
        match e {
            EffectDef::SetBasePowerToughness7bSourceEffect {
                power,
                toughness,
                duration: Duration::WhileOnBattlefield,
            } if src == id => Some((self.obj(src).entered_at, *power, *toughness)),
            EffectDef::SetBasePowerToughnessAttachedEffect { power, toughness } if self.attached(src) == Some(id) => {
                Some((self.obj(src).attached_at, *power, *toughness))
            }
            // an Aura's "enchanted … is a 1/1 Frog": its base power and toughness in timestamp order with the others
            EffectDef::BecomesCreatureAttachedEffect { token, .. } if self.attached(src) == Some(id) => {
                Some((self.obj(src).attached_at, token.power, token.toughness))
            }
            EffectDef::SetBasePowerToughnessAllEffect {
                power,
                toughness,
                duration: Duration::WhileOnBattlefield,
                filter,
            } if self.filter_matches(filter, you, Some(src), id) => {
                Some((self.obj(src).entered_at, self.value(power, you, src, 0), self.value(toughness, you, src, 0)))
            }
            EffectDef::ConditionalContinuousEffect { effect, condition } => {
                self.static_base_pt(effect, src, you, id).filter(|_| self.condition_holds(Some(condition), you, src))
            }
            _ => None,
        }
    }

    /// What a static ability's effect, from `src` controlled by `you`, adds to `id`'s power and toughness.
    fn static_boost(&self, e: &EffectDef, src: ObjId, you: Seat, id: ObjId) -> (i32, i32) {
        match e {
            EffectDef::BoostControlledEffect { power, toughness, filter, exclude_source, .. }
                if self.obj(id).controller == you
                    && !(*exclude_source && src == id)
                    && self.filter_matches(filter, you, Some(src), id) =>
            {
                (self.value(power, you, src, 0), self.value(toughness, you, src, 0))
            }
            EffectDef::BoostAllEffect { power, toughness, filter, exclude_source, .. }
                if !(*exclude_source && src == id) && self.filter_matches(filter, you, Some(src), id) =>
            {
                (self.value(power, you, src, 0), self.value(toughness, you, src, 0))
            }
            // what the effect would add first: its condition (Drowsing Tyrannodon's "a creature with power 4 or
            // greater") only for one that adds something, as Java checks it only in that effect's layer
            EffectDef::ConditionalContinuousEffect { effect, condition } => {
                let boost = self.static_boost(effect, src, you, id);
                if boost != (0, 0) && self.condition_holds(Some(condition), you, src) {
                    boost
                } else {
                    (0, 0)
                }
            }
            // a static "this gets +N/+N" (inside a conditional one)
            EffectDef::BoostSourceEffect { power, toughness, .. } if src == id => {
                (self.value(power, you, src, 0), self.value(toughness, you, src, 0))
            }
            EffectDef::BoostEnchantedEffect { power, toughness } if self.attached(src) == Some(id) => {
                (self.value(power, you, src, 0), self.value(toughness, you, src, 0))
            }
            _ => (0, 0),
        }
    }

    /// A stack object's name as Java gives it: the card's, or `stack ability (<rule text>)`.
    pub fn stack_name(&self, item: &StackItem) -> String {
        let card = self.obj(item.card);
        if let Some(d) = &item.delayed {
            return format!("stack ability ({})", d.rule);
        }
        match item.ability {
            Some((i, _)) => {
                let rule =
                    self.item_def(item).abilities.get(i).and_then(|a| a.on_stack()).map(|(r, ..)| r).unwrap_or("");
                format!("stack ability ({})", self.that_player_rule(item, rule))
            }
            // a face-down spell is still named by its card on the stack (`Spell.getName`)
            None => card.face_up.as_ref().map_or(card.name(), |f| f.name.as_str()).to_string(),
        }
    }

    /// `ThatPlayerControlsTargetAdjuster`: on the stack, the target's name gains the player it's about, "… that
    /// player controls (controlled by B)" (in a duel, the controller's opponent)
    pub fn that_player_rule(&self, item: &StackItem, rule: &str) -> String {
        let ability = item.ability.and_then(|(i, _)| self.item_def(item).abilities.get(i));
        if let Some(AbilityDef::TriggeredAbility { that_player_controls: true, that_player_name, .. }) = ability {
            let them = (item.controller + 1) % self.seats();
            let name = that_player_name.as_deref().unwrap_or("that player controls");
            let named = format!("{name} (controlled by {})", crate::log::player(them));
            return rule.replacen(name, &named, 1);
        }
        rule.to_string()
    }

    /// Takes an object out of whatever zone list holds it.
    pub fn remove_from_zone(&mut self, id: ObjId) {
        let (zone, owner) = {
            let o = self.obj(id);
            (o.zone, o.owner)
        };
        match zone {
            Zone::Library => self.player_mut(owner).library.retain(|x| *x != id),
            Zone::Hand => self.player_mut(owner).hand.retain(|x| *x != id),
            Zone::Graveyard => self.player_mut(owner).graveyard.retain(|x| *x != id),
            Zone::Battlefield => Arc::make_mut(&mut self.battlefield).retain(|x| *x != id),
            Zone::Exile => Arc::make_mut(&mut self.exile).retain(|x| *x != id),
            Zone::Command => Arc::make_mut(&mut self.command).retain(|x| *x != id),
            Zone::Outside => {
                let p = self.player_mut(owner);
                p.outside.retain(|x| *x != id);
                p.sideboard = p.sideboard.saturating_sub(1);
            }
            Zone::Stack => Arc::make_mut(&mut self.stack).retain(|x| x.card != id || x.ability.is_some()),
        }
    }

    /// Moves an object to a zone (its owner's, for hands, libraries and graveyards), at the end of that zone's list
    /// (a library's bottom). A permanent leaving the battlefield forgets its state.
    ///
    /// A creature going from the battlefield to a graveyard dies: its "when this dies" triggers wait. A token that
    /// leaves the battlefield is gone (it goes to no zone's list).
    pub fn move_to(&mut self, id: ObjId, zone: Zone) {
        let from = self.obj(id).zone;
        let attachments: Vec<(ObjId, u32)> = if from == Zone::Battlefield && zone != Zone::Battlefield {
            self.attachments(id).map(|a| (a, self.obj(a).zcc)).collect()
        } else {
            Vec::new()
        };
        let requested = zone;
        // a prepared permanent's copy is kept only while it stays: it can't be cast, and the next state check
        // removes it (704.5e)
        if from == Zone::Battlefield {
            if let Some(c) = self.obj_mut(id).prepared_copy.take() {
                if self.obj(c).zone == Zone::Exile {
                    self.obj_mut(c).may_play = None;
                }
            }
        }
        // flashback's exile replaces the move, whichever zone it was to (with Rest in Peace or a Leyline too, the
        // player chooses which applies)
        let flashed_to_graveyard = from == Zone::Stack && self.obj(id).flashback && zone == Zone::Graveyard;
        let zone = if from == Zone::Stack && self.obj(id).flashback { Zone::Exile } else { zone };
        // unearth: exiled instead, wherever it was going
        let zone = if from == Zone::Battlefield && self.obj(id).unearthed { Zone::Exile } else { zone };
        // madness (`MadnessReplacementEffect`): a card with madness going from a hand to a graveyard is exiled
        // instead, and its trigger fires (`MADNESS_CARD_EXILED`)
        let madness = from == Zone::Hand
            && zone == Zone::Graveyard
            && self.obj(id).def.abilities.iter().any(|a| matches!(a, AbilityDef::MadnessAbility { .. }));
        let zone = if madness { Zone::Exile } else { zone };
        // a disturb card's back face's "if this would be put into a graveyard from anywhere, exile it instead": the
        // back face's, on the battlefield or the stack; Java's card in a hand or library has both faces' abilities,
        // so it applies there too (Galedrifter discarded is exiled; dying as itself, it isn't)
        let disturb_exile =
            |d: &CardDef| d.abilities.iter().any(|a| matches!(a, AbilityDef::DisturbExileAbility { .. }));
        let zone = if zone == Zone::Graveyard
            && (disturb_exile(&self.obj(id).def)
                || (from != Zone::Battlefield && self.obj(id).def.transform_back.as_deref().is_some_and(disturb_exile)))
        {
            Zone::Exile
        } else {
            zone
        };
        // a spell's copy its own effect exiles (`ExileSpellEffect` moves the copy's card): it's in exile until the
        // next state check (704.5e), where a copy countered or moved any other way just leaves the stack
        let copy_exiled = from == Zone::Stack && zone == Zone::Exile && self.obj(id).flashback && self.obj(id).token;
        self.obj_mut(id).flashback = false;
        if !(from == Zone::Stack && zone == Zone::Battlefield) {
            self.obj_mut(id).escaped = false;
        }
        // Rest in Peace (two of them: the player chooses which applies)
        let rest_in_peace = self
            .battlefield
            .iter()
            .filter(|src| {
                self.obj(**src).def.abilities.iter().any(|a| match a {
                    AbilityDef::SimpleStaticAbility { effects, .. } => {
                        effects.iter().any(|e| matches!(e, EffectDef::GraveyardFromAnywhereExileReplacementEffect))
                    }
                    _ => false,
                })
            })
            .count();
        // Leyline of the Void: an opponent's card (not a token)
        let leylines = self
            .battlefield
            .iter()
            .filter(|src| {
                let s = self.obj(**src);
                s.controller != self.obj(id).owner
                    && !self.obj(id).token
                    && s.def.abilities.iter().any(|a| match a {
                        AbilityDef::SimpleStaticAbility { effects, .. } => {
                            effects.iter().any(|e| matches!(e, EffectDef::LeylineOfTheVoidEffect))
                        }
                        _ => false,
                    })
            })
            .count();
        // Valgavoth: a card (not a token) you didn't control, going to an opponent's graveyard, exiled with it
        // (one sacrificed earlier in the same action too: Java's short-living LKI, Liliana's "each player sacrifices")
        let valgavoths: Vec<ObjId> = if zone == Zone::Graveyard && !self.obj(id).token {
            self.battlefield
                .iter()
                .copied()
                .chain(self.short_lki.iter().copied().filter(|o| *o != id && !self.battlefield.contains(o)))
                .filter(|src| {
                    let s = self.obj(*src);
                    let o = self.obj(id);
                    // a spell's controller is its stack item's (a card cast from another player's exile)
                    let controller = if from == Zone::Stack {
                        self.stack
                            .iter()
                            .find(|i| i.card == id && i.ability.is_none())
                            .map(|i| i.controller)
                            .or(self.resolving_controller.filter(|(c, _)| *c == id).map(|(_, s)| s))
                            .unwrap_or(o.controller)
                    } else {
                        o.controller
                    };
                    o.owner != s.controller
                        && (!matches!(from, Zone::Battlefield | Zone::Stack) || controller != s.controller)
                        && s.def.abilities.iter().any(|a| match a {
                            AbilityDef::SimpleStaticAbility { effects, .. } => {
                                effects.iter().any(|e| matches!(e, EffectDef::ValgavothTerrorEaterReplacementEffect))
                            }
                            _ => false,
                        })
                })
                .collect()
        } else {
            Vec::new()
        };
        // "if that card would be put into a graveyard this turn, exile it instead"
        let graveyard_exiles =
            if zone == Zone::Graveyard { self.exile_if_graveyard.iter().filter(|x| **x == id).count() } else { 0 };
        let rest_in_peace = rest_in_peace + leylines + valgavoths.len() + graveyard_exiles;
        // static handlers' `ZONE_CHANGE` replacements ("exile it instead"), counted with the engine's graveyard ones
        // for the "which first" question
        let zone = if crate::rules::cards::any_replacement(self, crate::rules::cards::EventType::ZoneChange) {
            let o = self.obj(id);
            let player = if from == Zone::Battlefield { o.controller } else { o.owner };
            let mut ev = crate::rules::cards::Event {
                target: Some(id),
                from: Some(from),
                to: Some(zone),
                ..crate::rules::cards::Event::new(crate::rules::cards::EventType::ZoneChange, player)
            };
            let others = if zone == Zone::Graveyard { rest_in_peace } else { 0 };
            crate::rules::cards::replace_event(self, &mut ev, others);
            ev.to.unwrap_or(zone)
        } else {
            zone
        };
        let rest_in_peace = if zone == Zone::Graveyard { rest_in_peace } else { 0 };
        let valgavoths = if zone == Zone::Graveyard { valgavoths } else { Vec::new() };
        if let Some(&v) = valgavoths.first() {
            let zcc = self.obj(v).zcc;
            self.exiled_for_source.push((v, zcc, id));
        }
        if flashed_to_graveyard && rest_in_peace > 0 {
            let o = self.obj(id);
            self.owed_choices.push(o.owner);
        }
        // with an "if it would die this turn, exile it instead" too, the creature's controller chooses which applies
        // (Snarling Gorehound under Rest in Peace)
        let dies_exiles = if from == Zone::Battlefield && zone == Zone::Graveyard {
            let zcc = self.obj(id).zcc;
            self.exile_if_dies.iter().filter(|e| **e == (id, zcc)).count()
        } else {
            0
        };
        let zone = if zone == Zone::Graveyard && rest_in_peace > 0 {
            if rest_in_peace + dies_exiles > 1 {
                let o = self.obj(id);
                self.owed_choices.push(if from == Zone::Battlefield { o.controller } else { o.owner });
            }
            Zone::Exile
        } else {
            zone
        };
        // Etching of Kumano: a creature dealt damage this turn by a source its controller controlled is exiled, also
        // when the Etching leaves in the same action (Java looks back at it)
        let etchings = if from == Zone::Battlefield
            && zone == Zone::Graveyard
            && self.has_type(id, CardType::Creature)
            && self.obj(id).damaged_by.0 == self.turn
        {
            let mask = self.obj(id).damaged_by.1;
            let here = self.battlefield.iter().filter(|b| {
                let s = self.obj(**b);
                mask & (1 << s.controller) != 0 && self.is_etching(**b)
            });
            let left = self.etchings_left.iter().filter(|(e, c, t)| {
                *t == self.turn && mask & (1 << c) != 0 && self.lookback.contains(e) && !self.battlefield.contains(e)
            });
            here.count() + left.count()
        } else {
            0
        };
        // more than one applies: Java asks the creature's controller which, though each exiles it (two "exile it
        // instead" from Scorching Dragonfire and Shock's... the same replacement twice counts as two)
        let exiles = if from == Zone::Battlefield && zone == Zone::Graveyard {
            let zcc = self.obj(id).zcc;
            self.exile_if_dies.iter().filter(|e| **e == (id, zcc)).count()
        } else {
            0
        };
        if etchings + exiles > 1 {
            let c = self.obj(id).controller;
            self.owed_choices.push(c);
        }
        let etching = etchings > 0;
        if from == Zone::Battlefield && self.is_etching(id) {
            let (c, t) = (self.obj(id).controller, self.turn);
            self.etchings_left.retain(|(_, _, x)| *x == t);
            self.etchings_left.push((id, c, t));
        }
        // "if it would die this turn, exile it instead"
        let zone = if from == Zone::Battlefield
            && zone == Zone::Graveyard
            && (etching
                || self.exile_if_dies.contains(&(id, self.obj(id).zcc))
                || self.obj(id).exile_instead_of_dying
                // a finality counter
                || self.obj(id).counter("finality") > 0)
        {
            Zone::Exile
        } else {
            zone
        };
        // dies triggers look back at the battlefield as it was (this one included), and at what left it in the same
        // action (Java's short-living LKI: a sweeper's creatures see each other die)
        let died_controller = self.obj(id).controller;
        if zone == Zone::Exile && matches!(from, Zone::Graveyard | Zone::Battlefield) {
            self.exiled_batch = true;
        }
        if from == Zone::Battlefield && zone == Zone::Graveyard {
            let seen = (
                died_controller,
                self.has_type(id, CardType::Artifact),
                self.has_type(id, CardType::Creature),
                self.obj(id).token,
            );
            self.watch.dies.push(seen);
        }
        if from == Zone::Battlefield && zone == Zone::Graveyard && self.has_type(id, CardType::Creature) {
            self.creature_died = true;
            self.creatures_died += 1;
            self.watch.died.push(died_controller);
        }
        if zone == Zone::Exile {
            self.watch.exiled += 1;
        }
        // "whenever a card is put into a graveyard from anywhere" (`ZONE_CHANGE` to a graveyard): its source on the
        // battlefield now, a card (not a token) of the filter whose owner is `who`
        let into_graveyard = |g: &Game, src: ObjId, you: Seat, t: &Trigger| match t {
            Trigger::PutIntoGraveyardAll { filter, who } => {
                src != id
                    && g.obj(src).zone == Zone::Battlefield
                    && !g.obj(id).token
                    && Game::whose(who, you, g.obj(id).owner)
                    && g.filter_matches(filter, you, Some(src), id)
            }
            _ => false,
        };
        let dies = if from == Zone::Battlefield && zone == Zone::Graveyard {
            let mut sources: Vec<ObjId> =
                self.lookback.iter().copied().filter(|o| !self.battlefield.contains(o)).collect();
            sources.extend(self.battlefield.iter().copied());
            self.trigger_cause = Some(TriggerCause::Died(id));
            let fired = self.triggers(sources, |g, src, you, t| match t {
                Trigger::Dies
                | Trigger::EntersOrDies
                | Trigger::OpponentCastsOnYourTurnOrDies
                | Trigger::EntersOrLeaves
                | Trigger::LeavesBattlefield => src == id,
                Trigger::DiesIfCreature => src == id && g.has_type(id, CardType::Creature),
                // the source counts too unless it says "another", but it must match like the others
                // (Bishop of Wings, a Human, isn't "an Angel you control")
                Trigger::DiesAll { filter, include_self, this_unfiltered, .. } => {
                    (src == id && *this_unfiltered)
                        || (src != id || *include_self) && g.filter_matches(filter, you, Some(src), id)
                }
                Trigger::PeltCollector => src != id && died_controller == you,
                Trigger::DiesAttached { .. } => g.obj(src).attached_to.is_some_and(|(a, _)| a == id),
                Trigger::LeavesAll { filter } => g.filter_matches(filter, you, Some(src), id),
                Trigger::PutIntoGraveyardAll { .. } => into_graveyard(g, src, you, t),
                _ => false,
            });
            self.trigger_cause = None;
            let mut fired = self.pelt_collector(fired, id);
            // a source that left in the same action triggers as the permanent it was (Morbid Opportunist dying with
            // the others: its "once each turn" counts that object, not the card in the graveyard)
            for t in fired.iter_mut() {
                if t.card != id && !self.battlefield.contains(&t.card) {
                    if let Some((_, zcc)) = t.ability.as_mut() {
                        *zcc = zcc.saturating_sub(1);
                    }
                }
            }
            // blitz: its granted "When this creature dies, draw a card."
            if self.obj(id).blitzed {
                let delayed = Delayed {
                    rule: "When this creature dies, draw a card.".into(),
                    sacrifice: Vec::new(),
                    warp_exile: Vec::new(),
                    effects: vec![EffectDef::DrawCardSourceControllerEffect { amount: crate::card::Amount::Fixed(1) }],
                    source_zcc: self.obj(id).zcc,
                    fixed: Vec::new(),
                    return_tapped: Vec::new(),
                    targets: Vec::new(),
                    condition: None,
                };
                fired.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(id, died_controller) });
            }
            fired
        } else if from == Zone::Battlefield && zone != Zone::Battlefield {
            let sources: Vec<ObjId> = self.battlefield.iter().copied().collect();
            self.triggers(sources, |g, src, you, t| match t {
                Trigger::LeavesBattlefield | Trigger::EntersOrLeaves => src == id,
                Trigger::LeavesAll { filter } => g.filter_matches(filter, you, Some(src), id),
                _ => false,
            })
        } else if zone == Zone::Graveyard && from != Zone::Graveyard {
            let sources: Vec<ObjId> = self.battlefield.iter().copied().collect();
            self.triggers(sources, |g, src, you, t| into_graveyard(g, src, you, t))
        } else {
            Vec::new()
        };
        let p1p1 = self.obj(id).counters.iter().find(|(c, _)| c == "+1/+1").map_or(0, |c| c.1);
        let (power, toughness) = self.power_toughness(id);
        let mut dies = dies;
        for t in dies.iter_mut() {
            // "whenever a creature dies, … that card": a fixed target pointer at the card in the graveyard
            // (`DiesCreatureTriggeredAbility`'s setTargetPointer)
            // (the card it becomes in the graveyard: the move comes after its triggers are found); `PLAYER`: at
            // its controller
            if from == Zone::Battlefield
                && zone == Zone::Graveyard
                && matches!(self.trigger_of(t), Some(Trigger::DiesAll { .. }))
            {
                match t.ability.and_then(|(i, _)| self.item_def(t).abilities.get(i)) {
                    Some(crate::card::AbilityDef::TriggeredAbility { set_target_pointer: true, .. }) => {
                        t.targets = vec![vec![Target::Obj(id)]];
                        t.target_zcc = vec![vec![self.obj(id).zcc + 1]];
                    }
                    Some(crate::card::AbilityDef::TriggeredAbility { set_target_pointer_player: true, .. }) => {
                        t.targets = vec![vec![Target::Player(died_controller)]];
                        t.target_zcc = vec![vec![0]];
                    }
                    _ => {}
                }
            }
            if matches!(self.trigger_of(t), Some(Trigger::LeavesAll { .. })) {
                t.x = p1p1;
            }
            // its own leave triggers remember its power as it last was (`SourcePermanentPowerValue`)
            if t.card == id && t.lki_power.is_none() {
                t.lki_power = Some(power);
            }
            if t.card == id && t.lki_toughness.is_none() {
                t.lki_toughness = Some(toughness);
            }
            // a copy's (Mockingbird as Greedy Freebooter): the ability is the copied one, whatever the card is after
            // and one an effect gave it until end of turn (Fake Your Own Death's "When this creature dies, …"): the
            // card in the graveyard doesn't have it
            if t.card == id
                && t.ability.is_some()
                && t.def.is_none()
                && (self.obj(id).copied_def.is_some() || self.obj(id).land_type_base.is_some())
            {
                t.def = Some(self.obj(id).def.clone());
            }
        }
        // "when that creature dies this turn" (delayed triggers are checked before the others, in the order made)
        if from == Zone::Battlefield && zone == Zone::Graveyard && !self.when_dies.is_empty() {
            let (zcc, turn) = (self.obj(id).zcc, self.turn);
            self.when_dies.retain(|w| w.2 == turn);
            let (now, rest): (Vec<_>, Vec<_>) =
                std::mem::take(&mut self.when_dies).into_iter().partition(|w| w.0 == id && w.1 == zcc);
            self.when_dies = rest;
            for (k, (_, _, _, to_player, mut item)) in now.into_iter().enumerate() {
                if to_player {
                    item.targets = vec![vec![Target::Player(died_controller)]];
                    item.target_zcc = vec![vec![0]];
                }
                dies.insert(k, item);
            }
        }
        // earthbend's delayed trigger (delayed triggers are checked before the others)
        if from == Zone::Battlefield && matches!(zone, Zone::Graveyard | Zone::Exile) {
            let zcc = self.obj(id).zcc;
            // one for each time it was earthbent
            while let Some(k) = self.earthbent.iter().position(|e| *e == (id, zcc)) {
                self.earthbent.remove(k);
                let delayed = Delayed {
                    rule: "When it dies or is exiled, return it to the battlefield tapped.".into(),
                    sacrifice: Vec::new(),
                    warp_exile: Vec::new(),
                    effects: Vec::new(),
                    source_zcc: 0,
                    fixed: Vec::new(),
                    return_tapped: vec![(id, zcc + 1)],
                    targets: Vec::new(),
                    condition: None,
                };
                let owner = self.obj(id).owner;
                dies.insert(0, StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(id, owner) });
            }
        }
        if from == Zone::Battlefield && zone != Zone::Battlefield {
            let zcc = self.obj(id).zcc;
            let (back, kept): (Vec<_>, Vec<_>) =
                self.exiled_until_leaves.iter().copied().partition(|(s, z, _)| *s == id && *z == zcc);
            self.exiled_until_leaves = kept;
            let controller = self.obj(id).controller;
            self.returning.extend(back.into_iter().map(|(_, _, c)| (controller, c)));
        }
        if from == Zone::Battlefield && zone != Zone::Battlefield {
            let c = self.obj(id).controller;
            self.player_mut(c).permanent_left = true;
            if !self.has_type(id, CardType::Land) {
                self.void_this_turn = true;
            }
        }
        // what lost its abilities while this was on the battlefield gets them back
        if from == Zone::Battlefield {
            let zcc = self.obj(id).zcc;
            for b in self.battlefield.iter().copied().collect::<Vec<_>>() {
                if self.obj(b).lost_abilities.as_ref().is_some_and(|(s, z, _)| *s == id && *z == zcc) {
                    let o = self.obj_mut(b);
                    if let Some((_, _, def)) = o.lost_abilities.take() {
                        o.def = def;
                    }
                }
            }
            let o = self.obj_mut(id);
            if let Some((_, _, def)) = o.lost_abilities.take() {
                o.def = def;
            }
        }
        // its abilities waiting or on the stack keep the face they came from (a transformed card's leaving shows its
        // front again)
        if from == Zone::Battlefield && self.obj(id).front.is_some() {
            let face = self.obj(id).def.clone();
            for t in Arc::make_mut(&mut self.stack).iter_mut().chain(self.pending.iter_mut()) {
                if t.card == id && t.ability.is_some() && t.def.is_none() {
                    t.def = Some(face.clone());
                }
            }
        }
        let pt_lki = (from == Zone::Battlefield).then(|| self.power_toughness(id));
        if from == Zone::Battlefield {
            let kept = [AbilityDef::DeathtouchAbility, AbilityDef::LifelinkAbility]
                .into_iter()
                .filter(|a| self.has_ability(id, a))
                .collect();
            self.obj_mut(id).lki_keywords = kept;
            let o = self.obj_mut(id);
            // (a face-down one had none of its card's: its dies triggers don't fire)
            o.lki_def = (o.land_type_base.is_some() || o.face_up.is_some()).then(|| o.def.clone());
        } else {
            let o = self.obj_mut(id);
            o.lki_keywords.clear();
            o.lki_def = None;
        }
        self.remove_from_zone(id);
        // a transformed permanent's front, its card again after the effects that changed its back are undone
        let mut left_front = None;
        let mut left_face_up = None;
        {
            let o = self.obj_mut(id);
            o.toughness_lki = pt_lki.map(|p| p.1);
            o.power_lki = pt_lki.map(|p| p.0);
            // a face-down spell or permanent is its card again anywhere else (a face-down spell stays face down onto
            // the battlefield)
            if (from == Zone::Stack && zone != Zone::Battlefield) || from == Zone::Battlefield {
                if let Some(def) = o.face_up.take() {
                    o.def = def.clone();
                    left_face_up = Some(def);
                }
                o.face_down = false;
            }
            if from == Zone::Stack {
                if let Some(def) = o.adventurer.take() {
                    o.def = def;
                }
                // a spell cast transformed (a defeated Siege's back) is its front again anywhere but the battlefield
                if zone != Zone::Battlefield {
                    if let Some(def) = o.front.take() {
                        o.def = def;
                    }
                }
            }
            if from == Zone::Battlefield {
                o.cast_lki = o.cast;
                if let Some(def) = o.front.take() {
                    o.def = def.clone();
                    left_front = Some(def);
                }
                o.damaged_by = (0, 0);
            }
            o.on_adventure = false;
            o.flashback_turn = None;
            o.solved = false;
            o.exile_instead_of_dying = false;
            o.unearthed = false;
            o.may_play = None;
            o.may_play_end_step = false;
            o.may_play_while = None;
            o.any_mana = false;
            o.play_free = false;
            o.cast_only = false;
            o.chosen_name = None;
            o.info.clear();
            o.exiled_with = None;
            o.transformed_in_place = false;
            o.play_tax = 0;
            o.exerted = false;
            o.powered_up = false;
            o.marks.clear();
            o.creature_types = None;
            o.base_pt = None;
            o.regen_shields = 0;
            o.warp_from = None;
            if !(from == Zone::Stack && zone == Zone::Battlefield) {
                o.warped = false;
                o.evoked = false;
                o.impending = false;
                o.dashed = false;
                // kept as it last was on the battlefield (Vibrance's "if {G}{G} was spent" after its evoke sacrifice)
                if from != Zone::Stack && from != Zone::Battlefield {
                    o.spent_colors = [0; 5];
                }
                o.blitzed = false;
                o.offspring = false;
            }
        }
        let owner = self.obj(id).owner;
        if zone != Zone::Battlefield {
            let o = self.obj(id);
            if let Some(defender) = o.attacking {
                let left = (defender, o.attacking_pw.map(|p| p.0), o.blocked, o.blocked_by.clone());
                if let Some(g) = self.combat_groups.iter_mut().find(|g| g.attacker == id && g.left.is_none()) {
                    g.left = Some(left);
                }
            }
        }
        {
            let o = self.obj_mut(id);
            if zone != Zone::Battlefield {
                o.tapped = false;
                o.damage = 0;
                if from == Zone::Battlefield {
                    o.lki_counters = std::mem::take(&mut o.counters);
                }
                o.counters.clear();
                o.attacking = None;
                o.attacking_pw = None;
                o.loyalty_used = false;
                o.kicked = false;
                o.cast = false;
                o.lki_attached_to = if from == Zone::Battlefield { o.attached_to } else { None };
                o.lki_attachments = attachments;
                o.attached_to = None;
                o.uncounterable = false;
                o.foretold_turn = None;
                o.plotted_turn = None;
                o.suspended = false;
                if let Some((def, _, _)) = o.land_type_base.take() {
                    o.def = def;
                }
                if let Some(def) = o.becomes_base.take() {
                    o.def = def;
                }
                o.becomes_auras.clear();
                if let Some(def) = o.until_end_of_turn_def.take() {
                    o.def = def;
                }
                o.copied_def = None;
                o.eot_copy = None;
                if let Some(def) = o.pre_guildpact.take() {
                    o.def = def;
                    o.guildpact_order.clear();
                }
                if let Some(def) = o.printed_def.take() {
                    o.def = def;
                }
                // (Aetherwing crewed: its back as it was before it became a creature)
                if let Some(def) = left_front.take() {
                    o.def = def;
                }
                // a face-down one's card, whatever was given to its face-down 2/2 (backup's ability)
                if let Some(def) = left_face_up.take() {
                    o.def = def;
                }
                o.cast_x = 0;
                if from == Zone::Battlefield {
                    o.lki_class_level = o.class_level;
                }
                o.class_level = 1;
                o.class_gained.clear();
                o.blocking.clear();
                o.blocks = 0;
                o.blocked = false;
                o.blocked_by.clear();
                o.controller = o.owner;
                o.deathtouched = false;
            }
            if from != Zone::Battlefield && !matches!(zone, Zone::Battlefield | Zone::Stack) {
                o.adventure_zone = zone;
            }
            o.zone = zone;
            o.zcc += 1;
        }
        if self.events.is_some() {
            self.record(GameEvent {
                target: Some(Target::Obj(id)),
                source: self.resolving.map(|(_, card)| card),
                player: Some(owner),
                from: Some(from),
                to: Some(zone),
                ..GameEvent::new(EventKind::Zone)
            });
        }
        if from == Zone::Graveyard && zone != Zone::Graveyard {
            self.left_graveyard.push(id);
            self.watch.left_graveyard.push(owner);
        }
        if zone == Zone::Graveyard && from != Zone::Graveyard && !self.obj(id).token {
            let zcc = self.obj(id).zcc;
            self.watch.to_graveyard.push((id, zcc, from == Zone::Battlefield));
        }
        if madness && self.obj(id).zone == Zone::Exile {
            let fired = self.triggers([id], |_, src, _, t| src == id && matches!(t, Trigger::MadnessExiled));
            self.pending.extend(fired);
        }
        // DISCARDED_CARD, after the move (a card from a hand to a graveyard is a discarded one)
        if from == Zone::Hand && requested == Zone::Graveyard {
            self.discarded.push((owner, id));
            self.watch.discarded.push(owner);
            let sources = self.trigger_sources();
            let fired = self.triggers(sources, |g, src, you, t| match t {
                Trigger::Discards { who, filter } => {
                    Game::whose(who, you, owner) && g.filter_matches(filter, you, Some(src), id)
                }
                _ => false,
            });
            for mut t in fired {
                let a = t.ability.and_then(|(i, _)| self.item_def(&t).abilities.get(i));
                if matches!(a, Some(AbilityDef::TriggeredAbility { set_target_pointer_player: true, .. })) {
                    t.targets = vec![vec![Target::Player(owner)]];
                    t.target_zcc = vec![vec![0]];
                }
                self.pending.push(t);
            }
        }
        for mut t in dies {
            // "whenever one or more … die": once a batch, after the batch's other triggers
            if matches!(self.trigger_of(&t), Some(Trigger::DiesAll { batch: true, .. })) {
                if !self.batch_pending.iter().any(|b| b.card == t.card && b.ability == t.ability) {
                    self.batch_pending.push(t);
                }
                continue;
            }
            // a dead permanent's own trigger is its graveyard card's
            if t.card == id {
                t.ability = t.ability.map(|(i, _)| (i, self.obj(id).zcc));
            }
            // "that player": the dead permanent's controller
            let a = t.ability.and_then(|(i, _)| self.item_def(&t).abilities.get(i));
            if matches!(a, Some(AbilityDef::TriggeredAbility { set_target_pointer_player: true, .. })) {
                t.targets = vec![vec![Target::Player(died_controller)]];
                t.target_zcc = vec![vec![0]];
            }
            // "when this dies, return it …" (`SetTargetPointer.CARD`): the card in the graveyard
            let at_card = t.ability.and_then(|(i, _)| self.item_def(&t).abilities.get(i));
            if t.card == id
                && self.trigger_of(&t) == Some(&Trigger::Dies)
                && matches!(at_card, Some(AbilityDef::TriggeredAbility { set_target_pointer: true, .. }))
            {
                t.targets = vec![vec![Target::Obj(id)]];
                t.target_zcc = vec![vec![self.obj(id).zcc]];
            }
            // "return that card": the card in the graveyard
            if t.card != id && self.trigger_of(&t) == Some(&Trigger::DiesAttached { no_card: false }) {
                t.targets = vec![vec![Target::Obj(id)]];
                t.target_zcc = vec![vec![self.obj(id).zcc]];
            }
            self.pending.push(t);
        }
        if copy_exiled {
            Arc::make_mut(&mut self.exile).push(id);
        }
        if self.obj(id).token && zone != Zone::Battlefield {
            return;
        }
        match zone {
            Zone::Library => self.player_mut(owner).library.push(id),
            Zone::Hand => self.player_mut(owner).hand.push(id),
            Zone::Graveyard => {
                if self.obj(id).def.is_permanent_card() {
                    self.player_mut(owner).descended += 1;
                }
                self.player_mut(owner).graveyard.push(id)
            }
            Zone::Battlefield => {
                if !self.has_type(id, CardType::Land) {
                    let c = self.obj(id).controller;
                    self.player_mut(c).nonlands_entered += 1;
                }
                let seen = (
                    self.obj(id).controller,
                    self.has_type(id, CardType::Artifact),
                    self.has_type(id, CardType::Creature),
                    self.has_subtype(id, "Goblin"),
                );
                self.watch.entered.push(seen);
                let zcc = self.obj(id).zcc;
                self.watch.entered_permanents.push((id, zcc, self.obj(id).controller));
                let at = self.timestamp();
                let o = self.obj_mut(id);
                o.sick = true;
                o.entered_at = at;
                Arc::make_mut(&mut self.battlefield).push(id)
            }
            Zone::Exile => Arc::make_mut(&mut self.exile).push(id),
            Zone::Command => Arc::make_mut(&mut self.command).push(id),
            Zone::Stack => {}
            Zone::Outside => self.player_mut(owner).outside.push(id),
        }
    }
}

/// The ability a continuous effect gives (`GainAbility…Effect`)
fn given_ability(e: &EffectDef) -> Option<&AbilityDef> {
    match e {
        EffectDef::GainAbilityAllEffect { ability, .. }
        | EffectDef::GainAbilityControlledEffect { ability, .. }
        | EffectDef::GainAbilitySourceEffect { ability, .. }
        | EffectDef::GainAbilityAttachedEffect { ability } => Some(ability),
        EffectDef::ConditionalContinuousEffect { effect, .. } => given_ability(effect),
        _ => None,
    }
}

/// a basic mana ability with no other costs or effects, which Leyline of the Guildpact's any-color one replaces
fn guildpact_basic(a: &AbilityDef) -> bool {
    matches!(
        a,
        AbilityDef::WhiteManaAbility { effects, cost }
        | AbilityDef::BlueManaAbility { effects, cost }
        | AbilityDef::BlackManaAbility { effects, cost }
        | AbilityDef::RedManaAbility { effects, cost }
        | AbilityDef::GreenManaAbility { effects, cost }
            if effects.is_empty() && cost.is_empty()
    )
}
