//! Cards as data. Each ability, effect and target is tagged with the Java framework class it mirrors and carries that
//! class's constructor arguments, so `rust/tools/transpile.py` can write cards from `Mage.Sets` mechanically. A card
//! whose JSON names a class this engine doesn't have fails to load: it isn't in the engine.

use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize, PartialOrd, Ord)]
pub enum Color {
    W,
    U,
    B,
    R,
    G,
}

impl Color {
    pub const ALL: [Color; 5] = [Color::W, Color::U, Color::B, Color::R, Color::G];

    pub fn index(self) -> usize {
        self as usize
    }
}

/// Mana as an amount of each kind: white, blue, black, red, green, colorless.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Mana {
    pub colored: [u32; 5],
    pub colorless: u32,
}

impl Mana {
    pub fn of(color: Color, n: u32) -> Mana {
        let mut m = Mana::default();
        m.colored[color.index()] = n;
        m
    }

    pub fn total(&self) -> u32 {
        self.colored.iter().sum::<u32>() + self.colorless
    }
}

/// A mana cost: `{2}{R}{R}`, `{X}{G}`, `{C}`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ManaCost {
    pub generic: u32,
    pub colored: [u32; 5],
    pub colorless: u32,
    pub x: u32,
    /// of `colored`, the phyrexian symbols (`{W/P}`): each payable with 2 life instead
    pub phyrexian: [u32; 5],
    /// hybrid symbols (`{G/W}`): each paid with either color, the first tried first
    pub hybrid: Vec<(usize, usize)>,
    pub text: String,
}

impl ManaCost {
    pub fn parse(text: &str) -> Result<ManaCost, String> {
        let mut cost = ManaCost { text: text.to_string(), ..ManaCost::default() };
        let mut rest = text.trim();
        while !rest.is_empty() {
            if !rest.starts_with('{') {
                return Err(format!("bad mana cost {:?}", text));
            }
            let end = rest.find('}').ok_or_else(|| format!("bad mana cost {:?}", text))?;
            let symbol = &rest[1..end];
            match symbol {
                "W" => cost.colored[0] += 1,
                "U" => cost.colored[1] += 1,
                "B" => cost.colored[2] += 1,
                "R" => cost.colored[3] += 1,
                "G" => cost.colored[4] += 1,
                "C" => cost.colorless += 1,
                "X" => cost.x += 1,
                p if p.len() == 3 && &p[1..2] == "/" && "WUBRG".contains(&p[..1]) && "WUBRG".contains(&p[2..]) => {
                    cost.hybrid.push(("WUBRG".find(&p[..1]).unwrap(), "WUBRG".find(&p[2..]).unwrap()));
                }
                p if p.len() == 3 && p.ends_with("/P") => {
                    let k = "WUBRG".find(&p[..1]).ok_or_else(|| format!("mana symbol {{{}}} not supported", p))?;
                    cost.colored[k] += 1;
                    cost.phyrexian[k] += 1;
                }
                n => {
                    cost.generic += n.parse::<u32>().map_err(|_| format!("mana symbol {{{}}} not supported", n))?;
                }
            }
            rest = &rest[end + 1..];
        }
        Ok(cost)
    }

    pub fn mana_value(&self) -> u32 {
        self.generic + self.colored.iter().sum::<u32>() + self.colorless + self.hybrid.len() as u32
    }

    pub fn colors(&self) -> Vec<Color> {
        Color::ALL
            .iter()
            .copied()
            .filter(|c| {
                self.colored[c.index()] > 0 || self.hybrid.iter().any(|(a, b)| *a == c.index() || *b == c.index())
            })
            .collect()
    }

    /// the symbols of a color, hybrid ones included (devotion)
    /// The symbols of either color, a hybrid of both counted once (devotion to two colors)
    pub fn symbols_of_either(&self, c: Color, c2: Color) -> u32 {
        let (i, j) = (c.index(), c2.index());
        self.colored[i]
            + if i == j { 0 } else { self.colored[j] }
            + self.hybrid.iter().filter(|(a, b)| [*a, *b].iter().any(|x| *x == i || *x == j)).count() as u32
    }

    pub fn symbols_of(&self, c: Color) -> u32 {
        self.colored[c.index()] + self.hybrid.iter().filter(|(a, b)| *a == c.index() || *b == c.index()).count() as u32
    }

    pub fn is_empty(&self) -> bool {
        self.mana_value() == 0 && self.x == 0
    }

    /// this cost and another, as one (an additional cost added to a spell's)
    pub fn plus(&self, other: &ManaCost) -> ManaCost {
        let mut c = self.clone();
        c.generic += other.generic;
        for i in 0..5 {
            c.colored[i] += other.colored[i];
        }
        c.colorless += other.colorless;
        c.x += other.x;
        for i in 0..5 {
            c.phyrexian[i] += other.phyrexian[i];
        }
        c.hybrid.extend(other.hybrid.iter().copied());
        c.text.push_str(&other.text);
        c
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CardType {
    Artifact,
    Battle,
    Creature,
    Enchantment,
    Instant,
    Kindred,
    Land,
    Planeswalker,
    Sorcery,
}

/// Targets, named after the Java target classes. Every target of permanents is a `TargetPermanent` with a filter
/// (the Java classes for creatures, lands, "you control"… only fix the filter); `min` below `max` is "up to".
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(tag = "class")]
pub enum TargetDef {
    /// any target: a creature, player, planeswalker or battle
    TargetAnyTarget {
        #[serde(default = "one")]
        min: usize,
        #[serde(default = "one")]
        max: usize,
        /// "any other target": not the source
        #[serde(default)]
        another: bool,
        /// not one of the ability's earlier targets (`AnotherTargetPredicate`: Drakuseth's "other targets")
        #[serde(default)]
        not_first: bool,
        /// an amount divided among the targets as they're chosen (`TargetAnyTargetAmount`)
        #[serde(default)]
        distribute: Option<Amount>,
        /// `XTargetsCountAdjuster`: X targets (up to X when `min` is 0), the spell's X; none for X 0
        #[serde(default)]
        x_count: bool,
    },
    TargetPlayer,
    TargetOpponent,
    /// a spell on the stack, of the filter
    TargetSpell {
        #[serde(default)]
        filter: Filter,
    },
    /// an activated or triggered ability on the stack (`TargetActivatedOrTriggeredAbility`), named by its source
    /// Consign to Memory: a triggered ability or a colorless spell on the stack
    TargetTriggeredAbilityOrColorlessSpell,
    TargetStackAbility {
        #[serde(default = "one")]
        min: usize,
        #[serde(default = "one")]
        max: usize,
    },
    /// any other spell, or a permanent of the filter
    TargetSpellOrPermanent {
        #[serde(default)]
        filter: Filter,
    },
    /// a permanent of the filter
    TargetPermanent {
        #[serde(default)]
        filter: Filter,
        #[serde(default = "one")]
        min: usize,
        #[serde(default = "one")]
        max: usize,
        /// `TargetAmount`: this much is divided among the chosen targets as they're chosen (at most one target per
        /// point)
        #[serde(default)]
        distribute: Option<Amount>,
        /// `XTargetsCountAdjuster`: X targets (up to X when `min` is 0), the spell's X; none for X 0
        #[serde(default)]
        x_count: bool,
    },
    TargetPlayerOrPlaneswalker,
    /// an opponent or a planeswalker an opponent controls
    TargetOpponentOrPlaneswalker,
    /// a card in a graveyard (`TargetCardInGraveyard`, or `…InYourGraveyard` with the "you" owner predicate)
    TargetCardInGraveyard {
        #[serde(default)]
        filter: Filter,
        #[serde(default = "one")]
        min: usize,
        #[serde(default = "one")]
        max: usize,
        /// all from one graveyard: the first card's (`TargetCardInASingleGraveyard`)
        #[serde(default)]
        single: bool,
        /// up to the spell's X (Kozilek's Command's adjuster; X 0 is any number, as Java's max 0)
        #[serde(default)]
        max_x: bool,
        /// `XTargetsCountAdjuster`: X targets (up to X when `min` is 0), the spell's X; none for X 0
        #[serde(default)]
        x_count: bool,
    },
}

impl TargetDef {
    /// The same target for exactly one object or player (`chooseNewTarget`'s one target at a time)
    pub fn single(&self) -> TargetDef {
        let mut t = self.clone();
        match &mut t {
            TargetDef::TargetAnyTarget { min, max, .. }
            | TargetDef::TargetPermanent { min, max, .. }
            | TargetDef::TargetCardInGraveyard { min, max, .. } => {
                *min = 1;
                *max = 1;
            }
            _ => {}
        }
        t
    }
}

fn one_u32() -> u32 {
    1
}

fn one() -> usize {
    1
}

/// How long a continuous effect lasts (Java's `Duration`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum Duration {
    EndOfTurn,
    WhileOnBattlefield,
    UntilYourNextTurn,
}

/// `ComparisonType`
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Cmp {
    FewerThan,
    MoreThan,
    EqualTo,
    OrLess,
    OrGreater,
}

impl Cmp {
    pub fn holds(self, a: i32, b: i32) -> bool {
        match self {
            Cmp::FewerThan => a < b,
            Cmp::MoreThan => a > b,
            Cmp::EqualTo => a == b,
            Cmp::OrLess => a <= b,
            Cmp::OrGreater => a >= b,
        }
    }
}

/// One of a filter's predicates (Java's `Predicate`s). "You" is the filter's source's controller, or the player
/// choosing targets.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(tag = "p", rename_all = "snake_case")]
pub enum Pred {
    /// its mana cost is one of these (Urza's Saga's "{0} or {1}")
    ManaCostIn {
        costs: Vec<String>,
    },
    /// `ManaValueEqualToCountersSourceCountPredicate`: mana value equal to the source's counters of this kind (as it
    /// last was on the battlefield)
    ManaValueEqualSourceCounters {
        counter: String,
    },
    /// `CardType.X.getPredicate()`
    Type {
        t: CardType,
    },
    /// `SubType.X.getPredicate()` (the subtype's printed name)
    Subtype {
        s: String,
    },
    /// `SuperType.X.getPredicate()`: "BASIC", "LEGENDARY"
    Supertype {
        s: String,
    },
    /// `ColorPredicate`
    Color {
        c: Color,
    },
    /// `TargetController.X.getControllerPredicate()`: "you" or "opponent"
    Controller {
        who: String,
    },
    /// `TargetController.X.getOwnerPredicate()`
    Owner {
        who: String,
    },
    /// `AnotherPredicate`: not the source
    Another,
    /// `ColorlessPredicate`
    Colorless,
    /// `AuraAttachedPredicate`: an Aura other than the source is attached to it
    AuraAttached,
    /// controlled by the ability's first target (a player, or a planeswalker's controller), while targets are
    /// chosen; any controller before that (Searing Blaze's `SearingBlazeTarget`)
    ControlledByFirstTarget,
    /// `MulticoloredPredicate`: two or more colors
    Multicolored,
    /// a spell no mana was spent to cast (`FILTER_SPELL_NO_MANA_SPENT`)
    NoManaSpent,
    /// a kicked spell (Elfhame Druid's mana)
    KickedSpell,
    /// `ChosenNamePredicate`: named the source's chosen name
    ChosenName,
    /// `AdventurePredicate`: a card with an Adventure
    Adventure,
    /// `ManaValuePredicate`
    Mv {
        cmp: Cmp,
        n: i32,
    },
    Power {
        cmp: Cmp,
        n: i32,
    },
    Toughness {
        cmp: Cmp,
        n: i32,
    },
    /// `CastFromZonePredicate`: a spell cast from the zone ("graveyard", "exile", "hand"), or a card there
    CastFrom {
        zone: String,
    },
    /// `XManaValueTargetAdjuster`: mana value compared with the spell's X (the filter's source)
    ManaValueX {
        cmp: Cmp,
    },
    /// `TappedPredicate.TAPPED` (true) or `.UNTAPPED`
    Tapped {
        v: bool,
    },
    /// `AttackingPredicate`
    Attacking,
    /// mentor: power less than the source's (`MentorAbilityPredicate`)
    PowerLessThanSource,
    /// attacking or blocking
    InCombat,
    /// `TokenPredicate.TRUE` (true) or `.FALSE`
    Token {
        v: bool,
    },
    /// `CounterType.X.getPredicate()`: has at least one counter of the kind
    Counter {
        name: String,
        /// at least this many (`TargetHasCounterCondition`'s minimum)
        #[serde(default = "one_u32")]
        n: u32,
    },
    /// `ChosenSubtypePredicate`: of the creature type the source chose (`v`), or not
    ChosenSubtype {
        v: bool,
    },
    /// `AbilityPredicate(X.class)`: has the keyword
    Ability {
        a: Box<AbilityDef>,
    },
    /// `NamePredicate`
    Name {
        n: String,
    },
    /// `CutDownPredicate`: total power and toughness `n` or less
    TotalPowerToughness {
        n: i32,
    },
    /// `PermanentPredicate`: a permanent card (not an instant or sorcery)
    PermanentCard,
    /// `MonocoloredPredicate`: exactly one color
    Monocolored,
    /// `CounterAnyPredicate`: a counter of any kind on it
    CounterAny,
    /// `EnteredThisTurnPredicate`
    EnteredThisTurn,
    /// `EquippedPredicate`: an Equipment is attached to it
    Equipped,
    /// `EnchantedPredicate`: an Aura is attached to it
    Enchanted,
    /// `ModifiedPredicate`: a counter on it, an Equipment attached, or an Aura its controller controls
    Modified,
    /// `WasDealtDamageThisTurnPredicate`
    DamagedThisTurn,
    /// `BlockingPredicate`
    Blocking,
    /// `ManaSpentToCastPredicate`: the mana spent to cast it compared with `n`
    ManaSpent {
        cmp: Cmp,
        n: i32,
    },
    Not {
        x: Box<Pred>,
    },
    Or {
        xs: Vec<Pred>,
    },
    And {
        xs: Vec<Pred>,
    },
}

/// What a triggered ability triggers on. "This" is the ability's source; "you", its controller.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(tag = "e", rename_all = "snake_case", deny_unknown_fields)]
pub enum Trigger {
    /// this enters (`EntersBattlefieldTriggeredAbility`)
    Enters,
    /// the last defense counter was removed from this battle (`SiegeDefeatedTriggeredAbility`)
    SiegeDefeated,
    /// this enters or transforms into this face (`TransformsOrEntersTriggeredAbility`)
    TransformsOrEnters,
    /// this attacks (`AttacksTriggeredAbility`)
    Attacks,
    /// this attacks alone (`AttacksAloneSourceTriggeredAbility`)
    AttacksAlone,
    /// training: this attacks with a creature of greater power (`TrainingAbility`)
    Training,
    /// this dies (`DiesSourceTriggeredAbility`)
    Dies,
    /// dies, if it was a creature (the Enduring cycle's `EnduringGlimmerTriggeredAbility`)
    DiesIfCreature,
    EntersOrDies,
    /// an opponent casts a spell during your turn, or this dies (Voice of Resurgence's `OrTriggeredAbility`)
    OpponentCastsOnYourTurnOrDies,
    EntersOrAttacks,
    /// a permanent of the filter enters (`EntersBattlefieldAllTriggeredAbility`, `…ControlledTriggeredAbility`,
    /// landfall)
    EntersAll {
        filter: Filter,
        /// this too, whatever the filter ("this or another")
        #[serde(default)]
        include_self: bool,
        /// it triggers from its owner's graveyard instead of the battlefield (Bloodghast)
        #[serde(default)]
        graveyard: bool,
        /// "one or more": once for permanents entering together, after their other triggers (`ZONE_CHANGE_BATCH`)
        #[serde(default)]
        batch: bool,
    },
    /// a creature of the filter attacks (`AttacksCreatureYouControlTriggeredAbility`); with a target pointer, at it
    AttacksAll {
        filter: Filter,
        /// with a player target pointer: at the attacker's controller rather than the defending player
        #[serde(default)]
        attacker_controller: bool,
    },
    /// a creature of the filter attacks alone (`AttacksAloneControlledTriggeredAbility`, at `ATTACKER_DECLARED`);
    /// with a target pointer, at it
    AttacksAloneAll {
        filter: Filter,
    },
    /// the creature this is attached to attacks (`AttacksAttachedTriggeredAbility`); with a target pointer, at it
    AttacksAttached,
    /// this attacks (`ATTACKER_DECLARED`) or blocks (`CREATURE_BLOCKS`) (`AttacksOrBlocksTriggeredAbility`)
    AttacksOrBlocks,
    /// this blocks (`BlocksSourceTriggeredAbility`, `CREATURE_BLOCKS`)
    Blocks,
    /// a creature of the filter blocks (`BlocksAllTriggeredAbility`); with a target pointer, at it, or at its
    /// controller
    BlocksAll {
        filter: Filter,
    },
    /// this becomes blocked (`BecomesBlockedSourceTriggeredAbility`, `CREATURE_BLOCKED`); with a player target
    /// pointer, at the defending player
    BecomesBlocked,
    /// a creature of the filter becomes blocked (`BecomesBlockedAllTriggeredAbility`); with a target pointer, at it
    BecomesBlockedAll {
        filter: Filter,
    },
    /// `BLOCKER_DECLARED`, once for each blocker and attacker it blocks: this blocks a creature of the filter
    /// (`blocks`, `BlocksCreatureTriggeredAbility`) or is blocked by one (`blocked`,
    /// `BecomesBlockedByCreatureTriggeredAbility`); its target is that other creature
    BlockerDeclared {
        filter: Filter,
        #[serde(default)]
        blocks: bool,
        #[serde(default)]
        blocked: bool,
    },
    /// `who` ("you", "opponent") discards a card of the filter (`DISCARDED_CARD`); with a player target pointer, at
    /// that player
    Discards {
        who: String,
        #[serde(default)]
        filter: Filter,
    },
    /// a card of the filter is put into `who`'s graveyard from anywhere (`PutCardIntoGraveFromAnywhereAllTriggeredAbility`)
    PutIntoGraveyardAll {
        #[serde(default)]
        filter: Filter,
        who: String,
    },
    /// you discard one or more cards of the filter (`DiscardOneOrMoreCardsTriggeredAbility`, `DISCARDED_CARDS`): X is
    /// how many
    DiscardsOneOrMore {
        #[serde(default)]
        filter: Filter,
    },
    /// the creature this is attached to deals (combat) damage to a player of `who` ("you", "opponent", "any")
    /// (`DealsDamageToAPlayerAttachedTriggeredAbility`); with a target pointer, at that player; the damage is its X
    AttachedDealsDamageToPlayer {
        #[serde(default)]
        only_combat: bool,
        who: String,
    },
    /// one or more permanents of the filter you control deal (combat) damage to a player, once for each player's
    /// batch (`OneOrMoreDamagePlayerTriggeredAbility`); with a target pointer, at that player; the total is its X
    DamagePlayerBatch {
        filter: Filter,
        #[serde(default)]
        only_combat: bool,
    },
    /// any of `ts` (`OrTriggeredAbility`): it triggers once for an event that any of them triggers on
    Or {
        ts: Vec<Trigger>,
    },
    /// you cycle a card (`CycleControllerTriggeredAbility`, `ACTIVATED_ABILITY`)
    YouCycle,
    /// your first spell during each opponent's turn (`FirstSpellOpponentsTurnTriggeredAbility`)
    FirstSpellOpponentsTurn,
    /// this enters untapped (`EntersBattlefieldUntappedTriggeredAbility`)
    EntersUntapped,
    /// you scry or surveil (`ScryOrSurveilTriggeredAbility`, `SCRIED`/`SURVEILED`): X is the amount
    YouScryOrSurveil,
    /// one or more cards of the filter leave your graveyard (`CardsLeaveGraveyardTriggeredAbility`, once for each
    /// `ZONE_CHANGE_BATCH`)
    CardsLeaveGraveyard {
        #[serde(default)]
        filter: Filter,
    },
    /// `who` casts or copies a spell of the filter (`MagecraftAbility`)
    SpellCastOrCopied {
        filter: Filter,
        who: String,
    },
    /// you attack with `min` or more creatures of the filter (`AttacksWithCreaturesTriggeredAbility`)
    AttacksWith {
        min: usize,
        filter: Filter,
    },
    /// this leaves the battlefield
    LeavesBattlefield,
    /// heroic: you cast a spell that targets this (`HeroicAbility`)
    SpellCastTargetingThis,
    /// this becomes the target of a spell of the filter (`BecomesTargetSourceTriggeredAbility`); with a target
    /// pointer, at that spell's controller
    BecomesTargetOfSpell {
        filter: Filter,
    },
    /// `who` casts their second spell this turn
    CastSecondSpell {
        who: String,
    },
    /// this enters, and at the beginning of your upkeep (`OrTriggeredAbility`)
    EntersOrUpkeep,
    /// this Class becomes level `n`
    ClassLevel {
        n: u32,
    },
    /// you unlock this door, the left or the right (`UnlockThisDoorTriggeredAbility`)
    DoorUnlocked {
        left: bool,
    },
    /// you commit a crime: a spell or ability of yours targets an opponent, something of theirs or a card in their
    /// graveyard (`CommittedCrimeTriggeredAbility`)
    CommittedCrime,
    /// valiant: this becomes the target of a spell or ability you control for the first time each turn
    Valiant,
    /// you forage (`ForageTriggeredAbility`)
    Forage,
    /// this enters or leaves the battlefield (`EntersBattlefieldOrLeavesSourceTriggeredAbility`)
    EntersOrLeaves,
    /// you spend your `n`th total mana on spells this turn (`ExpendTriggeredAbility`)
    Expend {
        n: u32,
    },
    /// a player (`who`) sacrifices a permanent of the filter (`SacrificePermanentTriggeredAbility`)
    Sacrificed {
        filter: Filter,
        who: String,
    },
    /// `SacrificeOneOrMorePermanentsTriggeredAbility`: once for the permanents sacrificed together, from the batch
    /// event (after what triggers on each sacrifice and on them leaving)
    /// Ketramose: one or more cards put into exile from graveyards and/or the battlefield during your turn (once a batch)
    ExiledFromGraveyardOrBattlefieldBatch,
    /// "when you cast this spell" (`CastSourceTriggeredAbility`); `replicated`: only when its replicate cost was paid
    CastThis {
        #[serde(default)]
        replicated: bool,
    },
    SacrificedBatch {
        filter: Filter,
        who: String,
    },
    /// `who` ("you", "opponent") draws a card; with a target pointer, at that player
    Draws {
        who: String,
    },
    /// this is dealt damage (`DealtDamageToSourceTriggeredAbility`: once for each batch, with the damage as X)
    DealtDamage,
    /// you draw your `n`th card this turn (`DrawNthCardTriggeredAbility`)
    DrawsNth {
        n: u32,
    },
    /// this deals damage to an opponent (`only_combat`: combat damage)
    DealsDamageToOpponent {
        #[serde(default)]
        only_combat: bool,
    },
    /// the permanent this is attached to is put into a graveyard from the battlefield; its target is that card
    DiesAttached {
        /// no target pointer (`SetTargetPointer.NONE`): its target isn't the card
        #[serde(default)]
        no_card: bool,
    },
    /// a permanent of the filter leaves the battlefield; the trigger's X is the +1/+1 counters it had (The Ooze)
    LeavesAll {
        filter: Filter,
    },
    /// a permanent of the filter dies; with `include_self`, this too when it matches
    DiesAll {
        filter: Filter,
        #[serde(default)]
        include_self: bool,
        /// this triggers it whatever the filter says (`DiesThisOrAnotherTriggeredAbility`)
        #[serde(default)]
        this_unfiltered: bool,
    },
    /// at the beginning of `step` ("upkeep", "combat", "end") of `who`'s turn: "you", "opponent" or "any"
    StepBegins {
        step: String,
        who: String,
        /// it triggers from its owner's graveyard instead of the battlefield
        #[serde(default)]
        graveyard: bool,
        /// it triggers from exile, while the card is suspended (`SuspendUpkeepAbility`)
        #[serde(default)]
        suspended: bool,
    },
    /// the last time counter was removed from this suspended card in exile (`SuspendPlayCardAbility`)
    SuspendLastCounter,
    /// you gain life (`GainLifeControllerTriggeredAbility`)
    YouGainLife,
    /// `who` ("you", "opponent", "any") loses life (`LoseLifeTriggeredAbility`); the life lost is its X
    LosesLife {
        who: String,
    },
    /// one or more `counter` counters are put on this (Exemplar of Light)
    CountersAddedSource {
        counter: String,
    },
    /// a state trigger: this has `n` or more `counter` counters (Mazemind Tome)
    SourceCountersAtLeast {
        counter: String,
        n: i32,
    },
    /// you gain life for the first time each turn (`GainLifeFirstTimeTriggeredAbility`)
    YouGainLifeFirstTime,
    /// `who` ("you", "opponent", "any") casts a spell of the filter (`SpellCastControllerTriggeredAbility`, prowess)
    SpellCast {
        filter: Filter,
        who: String,
    },
    /// this card is cycled (`CycleTriggeredAbility`); its X is the cycling's
    Cycled,
    /// evolve: a creature you control enters with greater power or toughness than this one
    Evolve,
    /// this becomes exerted (`BecomesExertSourceTriggeredAbility`)
    Exerted,
    /// this crews a Vehicle (`CrewsVehicleSourceTriggeredAbility`); its target is the Vehicle
    CrewsVehicle,
    /// this becomes tapped (`BecomesTappedSourceTriggeredAbility`)
    BecomesTapped,
    /// this deals combat damage to a player
    CombatDamageToPlayer,
    /// this deals combat damage to a player or planeswalker (`DealsCombatDamageToAPlayerOrPlaneswalkerTriggeredAbility`)
    CombatDamageToPlayerOrPlaneswalker,
    /// a permanent of the filter deals combat damage to a player (`DealsDamageToAPlayerAllTriggeredAbility`): `who`
    /// is whose (empty: any player); a player pointer is the permanent's controller, or the damaged player when
    /// `damaged`
    CombatDamageToPlayerAll {
        filter: Filter,
        #[serde(default)]
        who: String,
        #[serde(default)]
        damaged: bool,
    },
    /// one or more opponents are dealt noncombat damage (Master of Barbs)
    OpponentsDealtNoncombatDamage,
    /// another permanent you control enters or dies, if its power is greater than this one's (Pelt Collector)
    PeltCollector,
    /// a creature or creature spell you control becomes the target of a spell or ability an opponent controls
    /// (Surrak, Elusive Hunter)
    SurrakTargeted,
    /// an opponent searches their library (Wan Shi Tong)
    OpponentSearchesLibrary,
    /// a Saga's chapter ability: lore counters were added that reached chapter `n`
    Chapter {
        n: u32,
    },
    /// battalion: this and at least two other creatures attack (`BattalionAbility`)
    Battalion,
    /// inspired: this becomes untapped (`InspiredAbility`, the `UNTAPPED` event)
    BecomesUntapped,
    /// eerie: an enchantment you control enters, or you fully unlock a Room (`EerieAbility`)
    Eerie,
}

/// Mana abilities as `ManaUtil.tryToAutoPay` tells them apart: a basic one (`{T}: Add {G}`, pain lands too), one of
/// any color, or another kind (which turns auto-pay off for that permanent).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManaKind {
    Basic,
    AnyColor,
    Other,
}

/// Conditions (Java's `Condition`s), checked for the controller of a source.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(tag = "c", rename_all = "snake_case", deny_unknown_fields)]
pub enum Condition {
    /// `SourceHasCountersCondition`: the source has a counter of any kind
    SourceHasCounters,
    /// `EnchantedSourceCondition` (an enchantment attached), or also an Equipment (Novice Knight's)
    SourceAttached {
        #[serde(default)]
        equipment: bool,
    },
    /// `SourceAttackingAloneCondition`: it's the only creature attacking
    SourceAttackingAlone,
    /// delirium: four or more card types among cards in your graveyard
    Delirium,
    /// `PlayLandCondition`: you played a land this turn
    PlayedLand,
    /// `CardsInExileCondition`: at least `n` cards in exile
    CardsInExile {
        n: usize,
    },
    /// `KickedCondition`: the spell was kicked (only a spell or what it put on the battlefield knows)
    Kicked,
    /// `OffspringCondition`: the permanent's offspring cost was paid
    Offspring,
    /// `EvokedCondition`: cast for its evoke cost
    Evoked,
    /// `TwoOfManaColorSpentCondition`: at least `n` mana of the color was spent to cast it
    ManaColorSpent {
        color: Color,
        n: u32,
    },
    /// `RaidCondition`: you attacked this turn
    Raid,
    /// a permanent you controlled left the battlefield this turn
    Revolt,
    /// you cast `n` or more instant and sorcery spells this turn
    InstantsSorceriesCast {
        n: u32,
    },
    /// the source, a spell being cast, targets something of the filter (`SourceTargetsPermanentCondition`)
    SourceTargets {
        filter: Filter,
    },
    /// `MyTurnCondition`
    MyTurn,
    /// `CastFromEverywhereSourceCondition`: this permanent was cast
    CastFromEverywhere,
    /// `LifeCompareCondition`: your life (or an opponent's) compared with `n`
    LifeCompare {
        who: String,
        cmp: Cmp,
        n: i32,
    },
    /// `PermanentsOnTheBattlefieldCondition` / `YouControlPermanentCondition`: the number of permanents of the filter
    /// (you control, unless `all`) compared with `n`
    Permanents {
        filter: Filter,
        cmp: Cmp,
        n: i32,
        #[serde(default)]
        all: bool,
    },
    /// `SourceHasCounterCondition`: the source has at least `n` counters of the kind
    SourceCounter {
        name: String,
        n: i32,
    },
    Not {
        x: Box<Condition>,
    },
    /// `CompoundCondition`: all of them
    And {
        xs: Vec<Condition>,
    },
    Or {
        xs: Vec<Condition>,
    },
    /// `SolvedSourceCondition.SOLVED`: the source is a solved Case
    Solved,
    /// a creature died this turn
    Morbid,
    /// you cast a non-Lemur creature spell with flying this turn (Momo's watcher)
    FlierCast,
    /// you gained life this turn
    YouGainedLife,
    /// `YouGainedLifeCondition`: the life you gained this turn compared with `n`
    LifeGained {
        cmp: Cmp,
        n: i32,
    },
    /// you drew `n` or more cards this turn
    DrewCards {
        n: u32,
    },
    /// four or more card types among cards exiled with the source (Keen-Eyed Curator)
    ExiledCardTypes {
        n: usize,
    },
    SourceEnteredThisTurn,
    /// `SourceAttackingCondition`
    SourceAttacking,
    /// renown's intervening "if": the source is renowned (`Permanent.isRenowned`)
    SourceRenowned,
    /// survival's intervening "if": the source is tapped
    SourceTapped,
    /// boast: the source attacked this turn (`BoastCondition`)
    SourceAttackedThisTurn,
    /// `AttackedOrBlockedThisCombatSourceCondition` (checked at end of combat, before it's removed from combat)
    AttackedOrBlockedThisCombat,
    /// `CardsInControllerGraveyardCondition`, spell mastery: at least `n` cards of the filter in your graveyard
    CardsInGraveyard {
        filter: Filter,
        n: i32,
    },
    /// you sacrificed at least `n` permanents this turn
    SacrificedThisTurn {
        n: i32,
    },
    /// `OpusCondition`: five or more mana was spent to cast the spell that set off the trigger
    Opus,
    /// the source Class is at least level `n`
    ClassLevel {
        n: u32,
    },
    /// `OpponentsLostLifeCondition`: an opponent lost life this turn
    OpponentsLostLife,
    /// `OpponentControlsMoreCondition`: an opponent controls more permanents of the filter than you
    OpponentControlsMore {
        filter: Filter,
    },
    /// `OpponentHasMoreLifeCondition`
    OpponentMoreLife,
    /// `OpponentHasMoreCardsInHandCondition`
    OpponentMoreCards,
    /// `HellbentCondition` (`n` 0), `HeckbentCondition` (`n` 1): you have at most `n` cards in hand
    CardsInHandAtMost {
        n: usize,
    },
    /// it's your first, second or third turn of the game (two players: your turns alternate)
    StartingTown,
    /// a value compared with `n` (`FormidableCondition`: your creatures' total power, eight or more)
    Value {
        of: Amount,
        cmp: Cmp,
        n: i32,
    },
    /// the source spell was cast from the zone ("graveyard", "hand"), or the source permanent entered as a spell
    /// cast from there (`CastFromGraveyardSourceCondition`, `CastFromHandSourcePermanentCondition`)
    CastFrom {
        zone: String,
    },
    /// an opponent has `n` or more poison counters (`CorruptedCondition`)
    OpponentPoison {
        n: i32,
    },
    /// an opponent has `n` or more cards in their graveyard (`CardsInOpponentGraveyardCondition`)
    OpponentGraveyard {
        n: usize,
    },
    /// a nonland permanent left the battlefield this turn or a spell was warped this turn (`VoidCondition`)
    Void,
    /// an Equipment is attached to the source (`EquippedSourceCondition`)
    SourceEquipped,
    /// the source permanent is tapped (`v`) or untapped (`SourceTappedCondition`)
    SourceTappedIs {
        v: bool,
    },
    /// the source (or it as it last was on the battlefield) passes the filter (`SourceMatchesFilterCondition`)
    SourceMatches {
        filter: Filter,
    },
    /// the permanent the source is attached to passes the filter (`AttachedToMatchesFilterCondition`)
    AttachedMatches {
        filter: Filter,
    },
}

/// Which objects a filter takes: those every predicate holds for. An empty filter takes everything.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct Filter(pub Vec<Pred>);

impl Filter {
    pub fn of(preds: Vec<Pred>) -> Filter {
        Filter(preds)
    }

    pub fn creatures() -> Filter {
        Filter(vec![Pred::Type { t: CardType::Creature }])
    }
}

/// Counters an effect puts on a permanent: `CounterType.P1P1.createInstance(2)`.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct CounterDef {
    pub name: String,
    pub amount: i32,
}

/// A token's characteristics, read from its Java token class.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct TokenDef {
    pub name: String,
    /// its Java class (`mage.game.permanent.token`), whose view and images the views use
    #[serde(default)]
    pub class: String,
    pub types: Vec<CardType>,
    #[serde(default)]
    pub colors: Vec<Color>,
    #[serde(default)]
    pub subtypes: Vec<String>,
    /// legendary (Cragflame)
    #[serde(default)]
    pub supertypes: Vec<String>,
    #[serde(default)]
    pub power: i32,
    #[serde(default)]
    pub toughness: i32,
    #[serde(default)]
    pub abilities: Vec<AbilityDef>,
    #[serde(default)]
    pub all_creature_types: bool,
    /// a planeswalker token's starting loyalty
    #[serde(default)]
    pub loyalty: Option<i32>,
    /// its images per set (`TokenRepository`): making one picks an image at random from its source's set's, or
    /// else from all of them, when there are two or more
    #[serde(default)]
    pub images: std::collections::BTreeMap<String, u32>,
}

impl TokenDef {
    /// How many images creating this token from a card of `set` picks among.
    pub fn image_choices(&self, set: &str) -> u32 {
        match self.images.get(set) {
            Some(n) => *n,
            None => self.images.values().sum(),
        }
    }

    pub fn card(&self) -> CardDef {
        CardDef {
            set: String::new(),
            name: self.name.clone(),
            cost: ManaCost::default(),
            types: self.types.clone(),
            subtypes: self.subtypes.clone(),
            supertypes: self.supertypes.clone(),
            colors: self.colors.clone(),
            power: self.power,
            toughness: self.toughness,
            abilities: self.abilities.clone(),
            spell: SpellDef::default(),
            loyalty: self.loyalty,
            defense: None,
            all_creature_types: self.all_creature_types,
            adventure: None,
            back: None,
            transform_back: None,
            omen: false,
            aftermath: false,
            prepare: None,
            room: Vec::new(),
            printed_cost: None,
            hints: Vec::new(),
            hint_abilities: Vec::new(),
        }
    }
}

fn yes() -> bool {
    true
}

/// Costs of an activated ability, named after the Java cost classes.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(tag = "class")]
pub enum CostDef {
    ManaCost {
        cost: String,
    },
    /// mill `amount` cards (only with that many in the library)
    MillCardsCost {
        amount: usize,
    },
    TapSourceCost,
    /// exert this: it doesn't untap during its controller's next untap step
    ExertSourceCost,
    SacrificeSourceCost,
    /// put (or remove) loyalty counters on the source
    PayLoyaltyCost {
        amount: i32,
    },
    /// sacrifice a permanent of the filter you control (chosen as the cost is paid)
    SacrificeTargetCost {
        filter: Filter,
        #[serde(default = "one")]
        amount: usize,
    },
    /// an additional cost: sacrifice any number (X, announced as the spell is cast) of the filter's permanents
    SacrificeXTargetCost {
        filter: Filter,
    },
    /// tap X untapped permanents of the filter you control, X announced (Merchant's Dockhand); the ability's X
    TapXTargetCost {
        filter: Filter,
    },
    /// tap `amount` permanents of the filter you control (chosen, not targeted)
    TapTargetCost {
        filter: Filter,
        amount: usize,
    },
    /// crew: tap any number of other untapped creatures you control with total power `value` or more
    CrewCost {
        value: i32,
    },
    /// exile the source
    ExileSourceCost,
    /// exile this card from your graveyard
    ExileSourceFromGraveCost,
    /// remove counters from the source
    RemoveCountersSourceCost {
        counter: CounterDef,
    },
    /// put counters on the source (Mazemind Tome's page counters)
    PutCountersSourceCost {
        counter: CounterDef,
    },
    /// `RemoveCounterCost(TargetControlledCreaturePermanent(1, max), …, amount)`: remove `amount` counters of the
    /// kind from among up to `max` permanents of the filter you control
    RemoveCountersFromAmongCost {
        filter: Filter,
        counter: String,
        amount: i32,
        max: usize,
    },
    /// return an unblocked attacker you control to its owner's hand (ninjutsu)
    ReturnAttackerToHandTargetCost,
    /// exile this many other cards of the filter from your graveyard (escape; `ExileFromGraveCost`)
    ExileFromGraveCost {
        amount: usize,
        #[serde(default)]
        filter: Filter,
    },
    /// one of these, the player's choice when both can be paid
    OrCost {
        costs: Vec<CostDef>,
    },
    /// discard a card of the filter
    DiscardCardCost {
        #[serde(default)]
        filter: Filter,
        /// how many (Cathartic Reunion's two)
        #[serde(default = "one")]
        amount: usize,
    },
    /// reveal between `min` and `max` cards of the filter from your hand; the number revealed is the ability's X
    /// (`RevealTargetFromHandCost`, `RevealTargetFromHandCostCount`)
    RevealTargetFromHandCost {
        #[serde(default)]
        filter: Filter,
        #[serde(default = "one")]
        min: usize,
        #[serde(default = "one")]
        max: usize,
    },
    /// exile a card of the filter from your hand (not this one)
    ExileFromHandCost {
        #[serde(default)]
        filter: Filter,
    },
    PayLifeCost {
        amount: i32,
    },
    /// pay {E} `amount` times: energy counters off the player
    PayEnergyCost {
        amount: i32,
    },
    /// harmonize: tapping the creature chosen to reduce the cost (`HarmonizeAbilityAdjuster`)
    HarmonizeCost,
    /// behold a permanent or card of a subtype: choose one you control, or reveal one from your hand
    BeholdCost {
        subtype: String,
    },
    /// discard this card (cycling)
    DiscardSourceCost,
    /// forage: exile three cards from your graveyard or sacrifice a Food, then "whenever you forage" triggers
    ForageCost,
}

fn put_hand() -> PutCards {
    PutCards::Hand
}

/// Where cards go (Java's `PutCards`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PutCards {
    Hand,
    Graveyard,
    Battlefield,
    BattlefieldTapped,
    Exiled,
    TopAny,
    BottomAny,
    BottomRandom,
}

/// A number an effect uses: fixed, or Java's `DynamicValue` worked out as the effect happens.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum Amount {
    Fixed(i32),
    Of(Value),
}

impl Amount {
    pub fn fixed(&self) -> Option<i32> {
        match self {
            Amount::Fixed(n) => Some(*n),
            Amount::Of(_) => None,
        }
    }
}

impl From<i32> for Amount {
    fn from(n: i32) -> Self {
        Amount::Fixed(n)
    }
}

/// Java's `DynamicValue`s, for the controller of the effect's source.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(tag = "v", rename_all = "snake_case", deny_unknown_fields)]
pub enum Value {
    /// the X paid (`GetXValue`, `ManacostVariableValue`)
    X,
    /// permanents of the filter (`PermanentsOnBattlefieldCount`, `CreaturesYouControlCount`…)
    Permanents {
        filter: Filter,
    },
    /// cards in the controller's hand
    CardsInHand,
    /// cards of the filter in the controller's graveyard
    CardsInGraveyard {
        filter: Filter,
    },
    /// the source's power (`SourcePermanentPowerValue`)
    SourcePower,
    /// the source's toughness (`SourcePermanentToughnessValue`)
    SourceToughness,
    /// the controller's life total (`ControllerLifeCount`)
    ControllerLife,
    /// the total power of the permanents of the filter
    TotalPower {
        filter: Filter,
    },
    /// counters of a kind on the source while it's on the battlefield
    SourceCounters {
        counter: String,
    },
    /// minus a value (`-X`)
    Negative {
        of: Box<Amount>,
    },
    /// a value times a number
    Times {
        of: Box<Amount>,
        n: i32,
    },
    /// a value halved, rounded down
    Half {
        of: Box<Amount>,
    },
    /// a value plus a number
    Plus {
        of: Box<Amount>,
        n: i32,
    },
    /// card types among cards in all graveyards (`CardTypesInGraveyardCount.ALL`)
    CardTypesInGraveyards,
    /// cards exiled with the source while it's on the battlefield (`UnlicensedHearseValue`)
    ExiledWithSource,
    /// the greatest mana value among permanents of the filter (`GreatestAmongPermanentsValue`)
    GreatestManaValue {
        filter: Filter,
    },
    /// the greatest power among permanents of the filter, 0 if there are none (`GreatestAmongPermanentsValue`)
    GreatestPower {
        filter: Filter,
    },
    /// cards the controller drew this turn
    CardsDrawn,
    /// life the controller gained this turn (`ControllerGainedLifeCount`)
    LifeGained,
    /// mana the controller spent on spells this turn (expend)
    ManaExpended,
    /// permanents the controller sacrificed this turn (`PermanentsSacrificedThisTurnCount.YOU`)
    SacrificedThisTurn,
    /// the total power, toughness or mana value of what the resolving spell or ability's costs sacrificed, as they
    /// last were on the battlefield (`SacrificeCostCreaturesPower`, `SacrificeCostCreaturesToughness`,
    /// `SacrificeCostManaValue.PERMANENT`)
    SacrificedPower,
    SacrificedToughness,
    SacrificedManaValue,
    /// opponents who lost life this turn (`PlayerLostLifeWatcher`)
    OpponentsLostLife,
    /// the controller's devotion to a color (and `c2`, for two): its mana symbols among the mana costs of permanents
    /// they control, a hybrid symbol of both counted once
    Devotion {
        c: Color,
        #[serde(default)]
        c2: Option<Color>,
    },
    /// basic land types among lands the controller controls (`DomainValue.REGULAR`)
    Domain,
    /// Auras attached to the source (`AuraAttachedCount`)
    AurasAttached,
    /// Equipment attached to the source (`EquipmentAttachedCount`)
    EquipmentAttached,
    /// card types among cards in the controller's graveyard (`CardTypesInGraveyardCount.YOU`)
    CardTypesInYourGraveyard,
    /// creatures that died this turn, anyone's (`CreaturesDiedThisTurnCount`)
    CreaturesDied,
    /// the controller's party: Clerics, Rogues, Warriors and Wizards they control, one of each, a creature counted
    /// for one type only (`PartyCount`)
    Party,
    /// colors among permanents of the filter (`ColorsAmongControlledPermanentsCount`)
    ColorsAmong {
        filter: Filter,
    },
    /// counters of every kind on the source, or on it as it last was on the battlefield (`CountersSourceCount.ANY`)
    SourceCountersAny,
    /// these values added up (`AdditiveDynamicValue`)
    Sum {
        xs: Vec<Amount>,
    },
    /// different powers among permanents of the filter (coven)
    DifferentPowers {
        filter: Filter,
    },
    /// nonland permanents that entered under the controller's control this turn (celebration)
    NonlandsEntered,
    /// permanent cards put into the controller's graveyard this turn (descend, `DescendedWatcher`)
    Descended,
}

/// One-shot effects, named after the Java effect classes.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(tag = "class", deny_unknown_fields)]
pub enum EffectDef {
    /// deals `amount` damage to the ability's target
    DamageTargetEffect {
        amount: Amount,
    },
    /// the controller draws `amount` cards
    DrawCardSourceControllerEffect {
        amount: Amount,
    },
    /// the controller gains `amount` life
    GainLifeEffect {
        amount: Amount,
    },
    /// destroys the target permanent
    DestroyTargetEffect,
    /// counters the target spell
    CounterTargetEffect,
    /// the target creature gets +power/+toughness
    BoostTargetEffect {
        power: Amount,
        toughness: Amount,
        duration: Duration,
    },
    /// the source gets +power/+toughness
    BoostSourceEffect {
        power: Amount,
        toughness: Amount,
        duration: Duration,
    },
    /// a static ability's effect: the permanents of `filter` its controller controls get +power/+toughness
    BoostControlledEffect {
        power: Amount,
        toughness: Amount,
        duration: Duration,
        filter: Filter,
        exclude_source: bool,
    },
    /// adapt N: N +1/+1 counters on the source if it has none
    AdaptEffect {
        amount: i32,
    },
    /// puts counters on the source
    AddCountersSourceEffect {
        counter: CounterDef,
        /// a number of counters worked out as it happens (the counter's own count when it's one); none when it's 0
        #[serde(default)]
        amount: Option<Amount>,
    },
    /// puts counters on the target permanent
    AddCountersTargetEffect {
        counter: CounterDef,
        /// how many, in place of the counter's own count
        #[serde(default)]
        value: Option<Amount>,
        /// on the ability's second target (`SecondTargetPointer`)
        #[serde(default)]
        second: bool,
    },
    /// removes one counter of the kind from the target
    RemoveCounterTargetEffect {
        counter: String,
    },
    /// the controller creates `amount` tokens; Kavaron Harrier's enter tapped and attacking and are sacrificed at end
    /// of combat
    /// Dark Salvation: the first target player creates X of the token, then the second target (up to one creature)
    /// gets -1/-1 until end of turn for each Zombie that player controls
    DarkSalvationEffect {
        token: TokenDef,
    },
    CreateTokenEffect {
        token: TokenDef,
        amount: Amount,
        #[serde(default)]
        tapped_and_attacking: bool,
        #[serde(default)]
        tapped: bool,
        /// attacking, untapped
        #[serde(default)]
        attacking: bool,
        #[serde(default)]
        sacrifice_at_end_of_combat: bool,
        /// Mobilize: sacrificed at the beginning of the next end step
        #[serde(default)]
        sacrifice_at_next_end_step: bool,
        #[serde(default)]
        haste_until_end_of_turn: bool,
        /// another token created with these (`withAdditionalTokens`)
        #[serde(default)]
        also: Option<TokenDef>,
        /// then this Equipment is attached to it (`CreateTokenAttachSourceEffect`); true: if its controller wants
        #[serde(default)]
        attach_source: Option<bool>,
    },
    /// deals damage equal to the source's power (as it last was on the battlefield) to the target
    DamageTargetSourcePowerEffect,
    /// a Food token; two if the creature sacrificed to pay for it had toughness 4 or greater
    WitchsOvenEffect {
        token: TokenDef,
    },
    /// `effect` if the condition holds, otherwise `otherwise`
    ConditionalOneShotEffect {
        #[serde(default)]
        effect: Option<Box<EffectDef>>,
        #[serde(default)]
        otherwise: Option<Box<EffectDef>>,
        condition: Condition,
        /// more effects done when it holds (`.addEffect(…)`)
        #[serde(default)]
        also: Vec<EffectDef>,
    },
    /// deals `amount` damage to each player of `controller` ("opponent")
    DamagePlayersEffect {
        amount: Amount,
        controller: String,
    },
    /// sacrifices the source
    SacrificeSourceEffect,
    /// the controller may pay `cost`; if they do, `effect`
    DoIfCostPaid {
        #[serde(default)]
        effect: Option<Box<EffectDef>>,
        cost: Vec<CostDef>,
        /// more effects done if it's paid (`.addEffect(…)`)
        #[serde(default)]
        also: Vec<EffectDef>,
        /// what's done if it isn't paid (Winternight Stories)
        #[serde(default)]
        otherwise: Vec<EffectDef>,
        /// false: paid without asking (`DoIfCostPaid(…, false)`)
        #[serde(default = "yes")]
        optional: bool,
    },
    /// the permanents of `filter` you control have `ability` (static), or gain it (resolving, until end of turn)
    GainAbilityControlledEffect {
        ability: Box<AbilityDef>,
        duration: Duration,
        filter: Filter,
        #[serde(default)]
        exclude_source: bool,
    },
    /// the target gains `ability`
    GainAbilityTargetEffect {
        ability: Box<AbilityDef>,
        duration: Duration,
    },
    /// the source has `ability` (static)
    GainAbilitySourceEffect {
        ability: Box<AbilityDef>,
        duration: Duration,
    },
    /// the permanents of the filter (as it resolves) lose the ability until end of turn (Shadowspear)
    LoseAbilityAllEffect {
        ability: Box<AbilityDef>,
        duration: Duration,
        filter: Filter,
    },
    /// a static effect that applies only while the condition holds ("my_turn")
    ConditionalContinuousEffect {
        effect: Box<EffectDef>,
        condition: Condition,
    },
    /// `effect` on each of the ability's targets in turn (`EachTargetPointer`: Dragonspark Reactor's damage to the
    /// player, then to the creature)
    EachTargetEffect {
        effect: Box<EffectDef>,
    },
    /// `effect` on the ability's target number `target` (0 the first; `SecondTargetPointer` is 1)
    TargetPointerEffect {
        target: usize,
        effect: Box<EffectDef>,
    },
    /// the source's base power and toughness are each a count (layer 7a; "creatures_you_control")
    SetBasePowerToughnessSourceEffect {
        count: String,
    },
    /// the source's base power and toughness are each `amount` (layer 7a; Pack Rat, Territorial Kavu)
    SetBasePowerToughnessAmountSourceEffect {
        amount: Amount,
    },
    /// if the target would die this turn, it's exiled instead
    ExileTargetIfDiesEffect,
    /// adds a subtype to the source: as a static ability while it holds, resolving until end of turn
    /// (`Game::added_subtypes`; subtypes aren't in the fingerprint)
    AddCardSubTypeSourceEffect {
        #[serde(default)]
        subtype: String,
    },
    /// as it enters, its controller may pay `life`; if they don't, it enters tapped
    TapSourceUnlessPaysEffect {
        life: i32,
    },
    /// as it enters, its controller may behold a `subtype`; if they don't, it enters tapped
    TapSourceUnlessBeholdEffect {
        subtype: String,
    },
    /// as it enters, its controller may reveal a card of the filter from their hand (`message`, the filter's text);
    /// if they don't, it enters tapped (the snarls and check lands' `RevealTargetFromHandCost`)
    TapSourceUnlessRevealsEffect {
        #[serde(default)]
        filter: Filter,
        message: String,
    },
    /// the first target deals damage equal to its power to the second (Bite Down)
    DamageWithPowerFromOneToAnotherTargetEffect,
    /// each opponent loses `amount` life
    LoseLifeOpponentsEffect {
        amount: Amount,
    },
    /// the controller loses `amount` life
    LoseLifeSourceControllerEffect {
        amount: Amount,
    },
    /// the target players lose `amount` life
    LoseLifeTargetEffect {
        amount: Amount,
    },
    /// destroys every permanent of `filter`
    DestroyAllEffect {
        filter: Filter,
    },
    /// deals `amount` damage to each permanent of `filter` the target player (or the target planeswalker's
    /// controller) controls
    DamageAllControlledTargetEffect {
        amount: Amount,
        filter: Filter,
    },
    /// put every card of `filter` in your graveyard onto the battlefield
    ReturnFromYourGraveyardToBattlefieldAllEffect {
        filter: Filter,
    },
    /// deals `amount` damage to every permanent of `filter`
    DamageAllEffect {
        amount: Amount,
        filter: Filter,
    },
    /// the permanents of `filter` get +power/+toughness (static: while on the battlefield; resolving: those there now)
    BoostAllEffect {
        power: Amount,
        toughness: Amount,
        duration: Duration,
        filter: Filter,
        exclude_source: bool,
    },
    /// puts counters on every permanent of `filter`
    AddCountersAllEffect {
        counter: CounterDef,
        filter: Filter,
        /// how many, if not the counter's own number
        #[serde(default)]
        amount: Option<Amount>,
    },
    /// the first target creature fights the second (each deals damage equal to its power to the other)
    FightTargetsEffect,
    /// The creature this Aura enchants (or enchanted as it left) fights the target.
    FightEnchantedTargetEffect,
    /// Mockingbird's enter-as-a-copy (done as it enters)
    MockingbirdEffect,
    /// until your next turn, creatures attacking your opponents get +2/+2 and trample
    GarrukCurseBreakerEffect,
    /// Static: the enchanted creature can't be blocked by more than one creature.
    CantBeBlockedByMoreThanOneAttachedEffect,
    /// destroy the target and every other permanent with its name
    DestroyAllNamedPermanentsEffect,
    /// double the source's counters of a kind
    DoubleCountersSourceEffect {
        name: String,
    },
    /// a static ability's effect in layer 7a: the source's base power is `amount`, its toughness that plus 1
    SetBasePowerToughnessPlusOneSourceEffect {
        amount: Amount,
    },
    /// each opponent loses `amount` life; you gain the life lost this way
    LoseLifeOpponentsYouGainLifeLostEffect {
        amount: Amount,
    },
    /// destroy the target creature if its mana value is `n` or less (`revolt_n` or less with revolt: Fatal Push)
    DestroyTargetIfManaValueEffect {
        n: i32,
        revolt_n: i32,
    },
    /// a static ability's effect: spells of the filter (cast by `who`, relative to its controller) cost {amount} more
    SpellsCostIncreasingAllEffect {
        amount: i32,
        filter: Filter,
        who: String,
    },
    /// spells of the filter you cast cost {amount} less (while the condition holds)
    SpellsCostReductionControllerEffect {
        filter: Filter,
        amount: i32,
        #[serde(default)]
        condition: Option<Condition>,
    },
    /// a static ability's effect: this spell costs {amount} less (if the condition holds; `stack_only`: only while
    /// it's being cast, not for the playable check)
    SpellCostReductionSourceEffect {
        amount: i32,
        #[serde(default)]
        condition: Option<Condition>,
        #[serde(default)]
        stack_only: bool,
    },
    /// you lose life equal to the target's mana value, then destroy it (Feed the Swarm)
    FeedTheSwarmEffect,
    /// 4 damage to the target; if a nonred permanent was dealt damage, you sacrifice a land (Redcap Melee)
    RedcapMeleeEffect,
    /// the first target deals damage equal to its power to the second; with trample, the excess to its controller
    RamThroughEffect,
    /// destroy each nonland permanent with mana value 2 or less; add {B} or {G} for each (Culling Ritual)
    CullingRitualEffect,
    /// 2 damage to any target; if a player was dealt damage, scry 1 (Play with Fire)
    PlayWithFireEffect,
    /// choose odd or even; exile each creature whose mana value is (Extinction Event)
    ExtinctionEventEffect,
    /// destroy all creatures; gain 1 life for each destroyed (Fumigate)
    FumigateEffect,
    /// this connives: its controller draws a card, then discards one; a nonland card discarded puts a +1/+1 counter
    /// on it
    ConniveSourceEffect,
    /// deals damage to the target player equal to the noncreature spells they cast this turn
    MagebaneLizardEffect,
    /// you may cast spells of the filter as though they had flash
    CastAsThoughItHadFlashAllEffect {
        filter: Filter,
    },
    /// until your next turn, you may cast spells of the filter as though they had flash (Teferi, Time Raveler's +1)
    CastAsThoughItHadFlashUntilYourNextTurnEffect {
        filter: Filter,
    },
    /// a static as-though effect (`CastAsThoughItHadFlash…Effect`) while a condition holds
    ConditionalAsThoughEffect {
        effect: Box<EffectDef>,
        condition: Condition,
    },
    /// a card's own "you may cast this spell as though it had flash" (from any zone; inside a conditional one)
    CastAsThoughItHadFlashSourceEffect,
    /// enchanted permanent can't attack or block, and its activated abilities (mana abilities too) can't be activated
    CantBlockAttackActivateAttachedEffect,
    /// this can't be blocked by more than one creature (`setMaxBlockedBy(1)`)
    CantBeBlockedByMoreThanOneSourceEffect,
    /// this can't block creatures of the filter
    CantBlockCreaturesSourceEffect {
        filter: Filter,
    },
    /// creatures of the filter can't block: a static, or until end of turn as it resolves. Either way the filter is
    /// looked at each time (Java's `RestrictionEffect` locks nothing in)
    CantBlockAllEffect {
        filter: Filter,
        duration: Duration,
    },
    /// creatures of the filter can't be blocked, as `CantBlockAllEffect`
    CantBeBlockedAllEffect {
        filter: Filter,
        duration: Duration,
    },
    /// puts the source's counters (as it last existed, when it's gone) on the target: each kind, or only `counter`
    PutSourceCountersOnTargetEffect {
        #[serde(default)]
        counter: Option<String>,
    },
    /// a static ability's effect: untap the source during each other player's untap step
    UntapSourceDuringEachOtherPlayersUntapStepEffect,
    /// a static ability's effect: untap the permanents of `filter` during each other player's untap step
    UntapAllDuringEachOtherPlayersUntapStepEffect {
        filter: Filter,
    },
    /// doubles the counters of the kind on each permanent of `filter`
    DoubleCounterOnEachPermanentEffect {
        name: String,
        filter: Filter,
    },
    /// doubles the counters of the kind on each target permanent
    DoubleCountersTargetEffect {
        name: String,
    },
    /// "if this is the `n`th time this ability has resolved this turn" (or `n`th or later), its effects
    IfAbilityHasResolvedXTimesEffect {
        n: u32,
        #[serde(default)]
        or_more: bool,
        effects: Vec<EffectDef>,
    },
    /// the creature the source is attached to gets +power/+toughness until end of turn (Adventuring Gear)
    BoostEquippedUntilEndOfTurnEffect {
        power: Amount,
        toughness: Amount,
    },
    /// the target permanents lose the ability until end of turn
    LoseAbilityTargetEffect {
        ability: Box<AbilityDef>,
    },
    /// the source loses the ability until end of turn
    LoseAbilitySourceEffect {
        ability: Box<AbilityDef>,
    },
    /// a static: the creature the source is attached to loses the ability
    LoseAbilityAttachedEffect {
        ability: Box<AbilityDef>,
    },
    /// the target creatures gain protection from the color of your choice until end of turn
    GainProtectionFromColorTargetEffect,
    /// the target permanents become the card types besides their others (until end of turn, or for good)
    AddCardTypeTargetEffect {
        types: Vec<CardType>,
        duration: Duration,
    },
    /// a static: the creature the source is attached to has the subtype besides its others
    AddCardSubtypeAttachedEffect {
        subtype: String,
    },
    /// target player gains `amount` life
    GainLifeTargetEffect {
        amount: Amount,
    },
    /// each player loses `amount` life, from the controller on in turn order
    LoseLifeAllPlayersEffect {
        amount: Amount,
    },
    /// the player the source is attacking loses `amount` life
    LoseLifeDefendingPlayerEffect {
        amount: Amount,
    },
    /// a static: your opponents can't gain life
    CantGainLifeOpponentsEffect,
    /// removes up to `counter.amount` counters of its kind from the source
    RemoveCounterSourceEffect {
        counter: CounterDef,
    },
    /// a static: creatures of the filter can't be blocked by more than one creature
    CantBeBlockedByMoreThanOneAllEffect {
        filter: Filter,
    },
    /// prevent all damage permanents of the filter (any source, with no filter) would deal this turn (Fog)
    PreventAllDamageByAllPermanentsEffect {
        #[serde(default)]
        filter: Option<Filter>,
        #[serde(default)]
        combat_only: bool,
    },
    /// prevent all damage that would be dealt to permanents of the filter (and to you, with `and_to_you`): a static,
    /// or until end of turn as it resolves
    PreventAllDamageToAllEffect {
        filter: Filter,
        duration: Duration,
        #[serde(default)]
        combat_only: bool,
        #[serde(default)]
        noncombat_only: bool,
        #[serde(default)]
        and_to_you: bool,
    },
    /// prevent all combat damage that would be dealt to the source, or by it with `by` (a static, or this turn)
    PreventCombatDamageToSourceEffect {
        duration: Duration,
        #[serde(default)]
        by: bool,
    },
    /// prevent the next `amount` damage (all of it, with none) that would be dealt to the targets this turn
    PreventDamageToTargetEffect {
        #[serde(default)]
        amount: Option<i32>,
        #[serde(default)]
        combat_only: bool,
    },
    /// layer 7b until end of turn: the target creatures have base power and toughness N/N
    SetBasePowerToughnessTargetEffect {
        power: Amount,
        toughness: Amount,
        /// for good (`Duration.EndOfGame`), not until end of turn
        #[serde(default)]
        permanent: bool,
    },
    /// layer 7b: the source has base power and toughness N/N, as a static (`WhileOnBattlefield`, usually in a
    /// conditional one) or until end of turn as it resolves
    SetBasePowerToughness7bSourceEffect {
        power: i32,
        toughness: i32,
        duration: Duration,
    },
    /// layer 7b: the enchanted or equipped creature has base power and toughness N/N
    SetBasePowerToughnessAttachedEffect {
        power: i32,
        toughness: i32,
    },
    /// layer 7b: the permanents of the filter, as a static; or the ones there as it resolves, until end of turn
    SetBasePowerToughnessAllEffect {
        power: Amount,
        toughness: Amount,
        duration: Duration,
        filter: Filter,
    },
    /// each opponent can cast spells only any time they could cast a sorcery (Teferi, Time Raveler)
    TeferiTimeRavelerEffect,
    /// return the target legendary creature card from your graveyard to the battlefield; it gains haste; exile it at
    /// the beginning of the next end step (Goryo's Vengeance)
    GoryosVengeanceEffect,
    /// reveal the top ten cards; for each card type, you may put a card of that type into your hand; the rest go on
    /// the bottom in a random order (Atraxa, Grand Unifier)
    AtraxaGrandUnifierEffect,
    /// exile the top `amount` cards (choosing one of them with `choose_one`); you may play them until the end of
    /// this turn (`EndOfTurn`) or of your next one (`UntilEndOfYourNextTurn`)
    ExileTopXMayPlayUntilEffect {
        amount: Amount,
        #[serde(default)]
        choose_one: bool,
        duration: String,
    },
    /// each opponent can't draw more than one card each turn
    NarsetParterOfVeilsEffect,
    /// you may play `amount` additional lands on each of your turns
    PlayAdditionalLandsControllerEffect {
        amount: u32,
    },
    /// you may play lands from your graveyard
    PlayLandsFromGraveyardControllerEffect,
    /// exiles the target card from a graveyard; if it was a creature card, `then` (a Clue)
    RavenEagleEffect {
        then: Box<EffectDef>,
    },
    /// "when that creature dies this turn, …" (`WhenTargetDiesDelayedTriggeredAbility`): armed on the first target
    /// as it is now; `to_player`: its effects' target is that creature's controller as it died
    CreateWhenTargetDiesDelayedEffect {
        effects: Vec<EffectDef>,
        rule: String,
        #[serde(default)]
        to_player: bool,
    },
    /// "at the beginning of the next end step, …" (`CreateDelayedTriggeredAbilityEffect` with
    /// `AtTheBeginOfNextEndStepDelayedTriggeredAbility`)
    CreateNextEndStepDelayedEffect {
        effects: Vec<EffectDef>,
        rule: String,
    },
    /// "… at the beginning of the next turn's upkeep" (`AtTheBeginOfNextUpkeepDelayedTriggeredAbility`)
    CreateNextUpkeepDelayedEffect {
        effects: Vec<EffectDef>,
        rule: String,
    },
    /// Expressive Iteration: of the top three cards, one into your hand, one on the bottom, one exiled (you may play
    /// it this turn)
    ExpressiveIterationEffect,
    /// look at the top card of target player's library (only its controller sees it)
    LookLibraryTopCardTargetPlayerEffect,
    /// "when you next cast a creature spell this turn, that creature enters with `amount` additional +1/+1
    /// counters" (`AddCounterNextSpellDelayedTriggeredAbility`)
    CreateNextCreatureSpellDelayedEffect {
        amount: i32,
        rule: String,
    },
    /// the target spell enters with `amount` additional +1/+1 counters
    AddCounterNextSpellEffect {
        amount: i32,
    },
    /// untap up to `amount` tapped lands
    UntapLandsEffect {
        amount: usize,
    },
    /// puts the target permanent into its owner's library `n`th from the top
    PutIntoLibraryNFromTopTargetEffect {
        n: usize,
    },
    /// if one or more tokens would be created under your control, twice that many are created instead
    CreateTwiceThatManyTokensEffect,
    /// deals X (the damage this was dealt) to the target; a player dealt damage this way can't gain life for the
    /// rest of the game
    ScreamingNemesisEffect,
    /// exiles the target cards from graveyards; for each creature card, you gain 1 life and each opponent loses 1
    /// (Graveyard Trespasser and its back face)
    GraveyardTrespasserEffect,
    /// transforms the source, if it's still that permanent
    TransformSourceEffect,
    /// the target gets +X/+0 until end of turn, X its power (Mightform Harmonizer: "double the power")
    MightformHarmonizerEffect,
    /// each creature card exiled with the source enters under your control with a flying counter, a 1/1 Spirit in
    /// addition to its other types
    GhostVacuumEffect,
    /// a token copy of the target, with haste, sacrificed at the beginning of the next end step
    ReflectionOfKikiJikiEffect,
    /// with one or fewer cards in your hand, each card you'd draw is two instead
    QuantumRiddlerReplacementEffect,
    /// noncombat damage from a source you control to an opponent or their permanent is 2 more (Artist's Talent)
    ArtistsTalentEffect {
        #[serde(default)]
        condition: Option<Condition>,
    },
    /// you have the ability (hexproof)
    GainAbilityControllerEffect {
        ability: Box<AbilityDef>,
    },
    /// "when you cast your next instant or sorcery spell this turn, copy that spell. You may choose new targets for
    /// the copy."
    CreateCopyNextSpellDelayedEffect,
    /// copy the spell the delayed trigger points at (new targets may be chosen)
    CopyTargetSpellEffect,
    /// exile your top card; you may cast it, and if you don't, 2 damage to each opponent
    ChandraTorchOfDefianceEffect,
    /// exile the target spell (into the source's exile zone)
    SpellQuellerEntersEffect,
    /// the card the source exiled: its owner may cast it without paying its mana cost
    SpellQuellerLeavesEffect,
    /// copy the spell that's this ability's source (casualty's "when you do"); its controller may choose new targets
    CopySourceSpellEffect,
    /// replicate's cast trigger: a copy of the spell for each time its replicate cost was paid
    ReplicateCopyEffect,
    /// a Class gains level `level`
    SetClassLevelEffect {
        level: u32,
    },
    /// exile the target card from a graveyard; a token copy of it, except a 4/4 black Zombie
    TheScarabGodExileEffect,
    /// "return it to its owner's hand at the beginning of the next end step", for the card that died
    ReturnSourceToHandNextEndStepEffect,
    /// as it enters: choose a basic land type (a land with `MultiversalPassagePassageEffect` becomes it)
    ChooseBasicLandTypeEffect,
    /// as it enters: choose a creature type (Cavern of Souls)
    ChooseCreatureTypeEffect,
    /// a spell its conditional mana was spent on can't be countered (Cavern of Souls; the mana marks the spell)
    CavernOfSoulsCantCounterEffect,
    /// "This land is the chosen type." (applied as the type is chosen)
    MultiversalPassagePassageEffect,
    /// "you may discard up to `amount` cards. If you do, draw that many cards"
    DiscardAndDrawThatManyEffect {
        amount: i32,
    },
    /// a static ability's effect: a creature dealt damage this turn by a source you controlled that would die is
    /// exiled instead
    EtchingOfKumanoReplacementEffect,
    /// exile this Saga, then return it to the battlefield transformed under your control
    ExileSagaAndReturnTransformedEffect,
    /// up to two creature cards with total mana value 4 or less from your graveyard onto the battlefield
    LivelyDirgeEffect,
    /// a 1/1 token copy of the source (offspring)
    OffspringEffect,
    /// the target loses all abilities for as long as the source remains on the battlefield
    LoseAllAbilitiesUntilSourceLeavesEffect,
    /// firebending: adds `amount` {R} that lasts until end of combat
    FirebendingEffect {
        amount: u32,
    },
    /// adds `amount` mana of one color its controller chooses (a resolving effect: always asked)
    AddManaOfAnyColorEffect {
        amount: u32,
    },
    /// a token that's a copy of the target permanent
    CreateTokenCopyTargetEffect,
    /// a token that's a copy of this permanent (or of it as it last was)
    CreateTokenCopySourceEffect,
    /// `EmpowerJaceEffect`: loyalty counters on a Jace token you control, first creating one if you control none
    EmpowerJaceEffect {
        amount: Amount,
        token: TokenDef,
    },
    /// earthbend N: the target land you control becomes a 0/0 creature with haste that's still a land, with N +1/+1
    /// counters; when it dies or is exiled, it returns to the battlefield tapped
    EarthbendTargetEffect {
        amount: Amount,
    },
    /// static: you may cast this card from your graveyard using its blitz ability
    TenaciousUnderdogEffect,
    /// static: creatures your opponents control can be targeted as though they didn't have hexproof
    NowhereToRunHexproofEffect,
    /// static: ward abilities of creatures your opponents control don't trigger
    NowhereToRunWardEffect,
    /// the source's creature types become these until it leaves the battlefield (`AddCardSubTypeSourceEffect`,
    /// `Duration.Custom`, not in addition)
    BecomesSubtypesSourceEffect {
        subtypes: Vec<String>,
    },
    /// the source has this base power and toughness until it leaves (layer 7b, `Duration.Custom`)
    SetBasePowerToughnessUntilLeavesEffect {
        power: i32,
        toughness: i32,
    },
    /// if the source is a Cleric: a deathtouch counter, and it becomes a Phyrexian Human Cleric with base 3/3
    EvolvedSleeperClericEffect,
    /// if the source is a Phyrexian: a +1/+1 counter, then you draw a card and lose 1 life
    EvolvedSleeperPhyrexianEffect,
    /// you have no maximum hand size (`MaximumHandSizeControllerEffect` set to `Integer.MAX_VALUE`)
    NoMaximumHandSizeEffect,
    /// destroy the target creature; if it wasn't attacking, its controller draws a card
    ProphesiedEndEffect,
    /// exile each opponent's graveyard
    SoulGuideLanternEffect,
    /// the attacking creature (the trigger's pointer) deals 1 damage to each opponent
    IngrisStingerquillEffect,
    /// other creatures you control enter with a +1/+1 counter for each opponent who lost life this turn
    GevScaledScorchEntersEffect,
    /// return the target creature card from your graveyard to the battlefield; it's exiled if it would die
    CantStayAwayEffect,
    /// activated abilities of artifacts, creatures and planeswalkers can't be activated
    ClarionConquerorEffect,
    /// return the target nonland permanent to its owner's hand; draw a card if you controlled it
    BoomerangBasicsEffect,
    /// deals damage to each player (you first) equal to the nonbasic lands they control
    SunspineLynxEffect,
    /// choose a card of the filter in your graveyard (if there is one) and return it to your hand
    ReturnCardChosenFromGraveyardEffect {
        filter: Filter,
        optional: bool,
    },
    /// reveal the top `amount`, put `pick` of the filter into your hand (asked first if optional), the rest into
    /// your graveyard
    RevealLibraryPickControllerEffect {
        amount: Amount,
        pick: usize,
        filter: Filter,
        optional: bool,
        /// where the picked cards go (the rest go to the graveyard)
        #[serde(default = "put_hand")]
        to: PutCards,
    },
    /// put the target card from a graveyard on top (or the bottom) of its owner's library
    PutOnLibraryTargetEffect {
        top: bool,
    },
    /// if you would gain life, you gain that much plus 1 instead
    GainPlusOneLifeReplacementEffect,
    /// the target player draws `draw` cards, then discards `discard`
    DrawDiscardTargetEffect {
        draw: i32,
        discard: i32,
    },
    /// the target player creates a token
    CreateTokenTargetEffect {
        token: TokenDef,
        /// how many each target player creates (one when absent)
        #[serde(default)]
        amount: Option<Amount>,
    },
    /// the target player scries `amount`
    ScryTargetEffect {
        amount: Amount,
    },
    /// proliferate: the controller chooses any number of permanents and players with counters, then each gets
    /// another counter of each kind already there
    ProliferateEffect,
    /// populate: a token that's a copy of a creature token the controller chooses among theirs
    PopulateEffect,
    /// as long as `amount` is less than `less_than`, this isn't a creature (a Theros god's devotion)
    LoseCreatureTypeSourceEffect {
        amount: Amount,
        less_than: i32,
    },
    /// the permanents of `filter` attack each combat if able (a static, `AttacksIfAbleAllEffect`)
    AttacksIfAbleAllEffect {
        filter: Filter,
    },
    /// amass: with no Army, create `token` (a 0/0 Army) first; then `amount` +1/+1 counters on an Army the
    /// controller chooses, which becomes a `subtype` too
    AmassEffect {
        amount: Amount,
        subtype: String,
        token: TokenDef,
    },
    /// the targets don't untap during their controller's next untap step
    DontUntapInControllersNextUntapStepTargetEffect,
    /// you win the game: each opponent loses it (`PlayerImpl.won`)
    WinGameSourceControllerEffect,
    /// bolster: `amount` +1/+1 counters on the creature you control with the least toughness (chosen among ties)
    BolsterEffect {
        amount: Amount,
    },
    /// renown: `amount` +1/+1 counters on the source, which becomes renowned (`BecomesRenownedSourceEffect`)
    BecomesRenownedSourceEffect {
        amount: i32,
    },
    /// read ahead: a Saga enters with the number of lore counters its controller chooses, 1 to `max`
    /// (`SagaLoreCountersEffect`)
    SagaReadAheadEffect {
        max: u32,
    },
    /// riot: the creature enters with a +1/+1 counter or, the controller's choice, haste (`RiotReplacementEffect`)
    RiotEffect,
    /// fabricate: `amount` +1/+1 counters on the source or, the controller's choice, `amount` Servo tokens
    FabricateEffect {
        amount: i32,
        token: TokenDef,
    },
    /// endure: `amount` +1/+1 counters on the source while it's still there or, the controller's choice, an
    /// `amount`/`amount` Spirit `token` (`EndureSourceEffect.doEndure`)
    EndureSourceEffect {
        amount: Amount,
        token: TokenDef,
    },
    /// decayed's "sacrifice it at end of combat" (`AtTheEndOfCombatDelayedTriggeredAbility`)
    SacrificeSourceAtEndOfCombatEffect,
    /// unleash: it may enter with a +1/+1 counter (`UnleashReplacementEffect`); with one, it can't block
    UnleashEffect,
    /// recruit: draw a card, then discard a card; a nonland card discarded makes a `token` (a Human Soldier)
    RecruitEffect {
        token: TokenDef,
    },
    /// connive: the target's controller draws a card, then discards a card; a +1/+1 counter on it for each nonland
    /// card discarded (`ConniveSourceEffect.connive`)
    ConniveTargetEffect,
    /// Blade of the Bloodchief: a +1/+1 counter on the equipped creature, two if it's a Vampire
    BladeOfTheBloodchiefEffect,
    /// Edifice of Authority: until your next turn, the target can't attack or block and its activated abilities
    /// can't be activated
    EdificeOfAuthorityEffect,
    /// tap the permanent the source is attached to
    TapEnchantedEffect,
    /// prevent all damage that would be dealt to the source this turn
    PreventAllDamageToSourceEffect,
    /// choose a permanent of the filter: prevent all combat damage it would deal this turn (Ajani's Aid)
    PreventCombatDamageByChosenEffect {
        filter: Filter,
    },
    /// tap every permanent of the filter
    TapAllEffect {
        filter: Filter,
    },
    /// reveal cards from the top until one of the filter: it goes to your hand, the rest on the bottom at random
    RevealUntilToHandEffect {
        filter: Filter,
    },
    /// gain control of every permanent of the filter the target player controls (for the rest of the game)
    GainControlAllControlledTargetEffect {
        filter: Filter,
    },
    /// gain control of the target permanents for `duration` (EndOfTurn, Forever, WhileControlled, UntilSourceLeaves)
    GainControlTargetEffect {
        duration: String,
    },
    /// the first target's controller (as it last was) is dealt `amount` damage
    DamageTargetControllerEffect {
        amount: Amount,
    },
    /// the first target's controller loses `amount` life
    LoseLifeTargetControllerEffect {
        amount: Amount,
    },
    /// the first target's controller draws `amount` cards
    DrawCardTargetControllerEffect {
        amount: Amount,
    },
    /// `first` damage to the target permanent and `second` to its controller
    DamageTargetAndTargetControllerEffect {
        first: i32,
        second: i32,
    },
    /// you sacrifice `amount` permanents of the filter
    SacrificeControllerEffect {
        filter: Filter,
        amount: Amount,
    },
    /// return `amount` permanents of the filter you control to their owner's hand
    ReturnToHandChosenControlledPermanentEffect {
        filter: Filter,
        amount: usize,
    },
    /// reveal the top two: the lands onto the battlefield, the rest into your hand
    NissaNaturesArtisanEffect,
    /// exile the top card; until end of turn you may play it without paying its mana cost
    OraclesVaultFreeEffect,
    /// look at the top X cards: one into your hand, the rest on the bottom in any order
    MerchantsDockhandEffect,
    /// remove the flood counters; none removed: a flood counter and draw a card, else add {C}{G}{U}
    BountyOfTheLuxaEffect,
    /// Believe: the top card of your library onto the battlefield if it's a creature card and you choose to, else
    /// into your hand
    BelieveEffect,
    /// `UntapTargetEffect` with `EachTargetPointer`: every target's permanents untap
    UntapEachTargetEffect,
    /// `ReturnToHandTargetEffect` with an `EachTargetPointer`: every target of each of its targets (Aether Tradewinds)
    ReturnToHandEachTargetEffect,
    /// Nissa's Encouragement: a Forest, a Brambleweft Behemoth and a Nissa, Genesis Mage from your library (and the
    /// ones not found there, from your graveyard) into your hand
    NissasEncouragementEffect,
    /// sacrifice the target at the beginning of the next end step (Hazoret's Favor)
    SacrificeTargetNextEndStepEffect,
    /// reveal the top card: a permanent card may go onto the battlefield, else it may go on the bottom
    AidFromTheCowlEffect,
    /// reveal until a nonland card with mana value 3 or less, which may be cast free; the rest go on the bottom
    TreasureKeeperEffect,
    /// the permanent the source is attached to doesn't untap in its controller's untap step (a static's)
    DontUntapInControllersUntapStepEnchantedEffect,
    /// discard your hand, then draw that many cards
    DiscardHandDrawSameNumberSourceEffect,
    /// this must be blocked if able (a static's)
    MustBeBlockedByAtLeastOneSourceEffect,
    /// untap each permanent of the filter
    UntapAllEffect {
        filter: Filter,
    },
    /// this can't attack alone (a static's)
    CantAttackAloneSourceEffect,
    /// this can't block alone (a static's)
    CantBlockAloneSourceEffect,
    /// you may tap the target permanent, or untap it if it's tapped
    MayTapOrUntapTargetEffect,
    /// you get `amount` {E} (energy counters)
    GetEnergyCountersControllerEffect {
        amount: Amount,
    },
    /// search your library and/or graveyard for a card of the filter, reveal it, put it into your hand; shuffle if
    /// the library was searched (`both`: the library is searched without asking)
    SearchLibraryGraveyardPutInHandEffect {
        filter: Filter,
        #[serde(default)]
        both: bool,
    },
    /// Shifting Woodland: the source becomes a copy of the target card in your graveyard until end of turn
    ShiftingWoodlandCopyEffect,
    /// Emrakul, the Promised End: you control the target opponent during their next turn; then they take an extra
    /// turn
    EmrakulThePromisedEndGainControlEffect,
    /// Ugin's Labyrinth: you may exile a colorless card with mana value 7 or more from your hand, with the source
    UginsLabyrinthExileEffect,
    /// Ugin's Labyrinth: the cards exiled with the source go to their owner's hand
    UginsLabyrinthReturnEffect,
    /// adds mana to the controller's pool (WUBRG, then colorless)
    BasicManaEffect {
        colored: [u32; 5],
        colorless: u32,
    },
    /// the target permanent's controller (as it last was) creates `amount` tokens
    CreateTokenControllerTargetEffect {
        token: TokenDef,
        amount: u32,
        tapped: bool,
    },
    /// returns every permanent of the filter to its owner's hand
    ReturnToHandFromBattlefieldAllEffect {
        filter: Filter,
    },
    /// the controller discards their hand
    DiscardHandControllerEffect,
    /// mill `amount`, then put a milled card of the filter into your hand (up to one if optional)
    MillThenPutInHandEffect {
        amount: Amount,
        filter: Filter,
        optional: bool,
        /// what happens if no card is put into the hand (Ainok Wayfarer's +1/+1 counter)
        #[serde(default)]
        otherwise: Option<Box<EffectDef>>,
    },
    /// a spell costs `per` less for each of `amount` (`SpellCostReductionForEachSourceEffect`)
    SpellCostReductionForEachSourceEffect {
        per: i32,
        amount: Amount,
    },
    /// Lovestruck Beast: this can't attack unless you control a 1/1 creature
    LovestruckBeastEffect,
    /// the target opponent exiles a creature they control and their graveyard (Strategic Betrayal)
    StrategicBetrayalEffect,
    /// the target opponent sacrifices a creature, an enchantment and a planeswalker; for each they can't, they lose
    /// 2 life and you draw (Invoke Despair)
    InvokeDespairEffect,
    /// discard a card unless you attacked this turn (Chart a Course)
    ChartACourseEffect,
    /// exile each other nonland permanent with mana value 2 or less until this leaves (Temporary Lockdown)
    TemporaryLockdownEffect,
    /// exile all artifacts and creatures with mana value 2 or less until this leaves
    PinnacleStarcageExileEffect,
    /// the controller may pay `cost` (asked with `message`); when they do, a reflexive trigger (`DoWhenCostPaid`)
    DoWhenCostPaid {
        cost: Vec<CostDef>,
        message: String,
        rule: String,
        #[serde(default)]
        targets: Vec<TargetDef>,
        effects: Vec<EffectDef>,
        /// false: paid without asking (`DoWhenCostPaid(…, false)`)
        #[serde(default = "yes")]
        optional: bool,
    },
    /// a counter on the source (while it's still there), then a reflexive trigger: "when you do, if …"
    AddCounterSourceReflexiveEffect {
        counter: CounterDef,
        rule: String,
        #[serde(default)]
        condition: Option<Condition>,
        #[serde(default)]
        targets: Vec<TargetDef>,
        effects: Vec<EffectDef>,
    },
    /// the cards it exiled go to their owners' graveyards, then a `token` for each one that did
    PinnacleStarcageTokenEffect {
        token: TokenDef,
    },
    /// a static ability's effect: spells of the filter you control can't be countered
    CantBeCounteredControlledEffect {
        filter: Filter,
        duration: Duration,
    },
    /// a static ability's effect: creatures entering the battlefield don't cause abilities to trigger (Torpor Orb)
    DontCauseTriggerEffect,
    /// a static ability's effect: each player of `who` can't cast more than one spell each turn
    CantCastMoreThanOneSpellEffect {
        who: String,
    },
    /// your opponents can't cast spells during your turn (Voice of Victory)
    CantCastDuringYourTurnEffect,
    /// equip abilities you activate that target this cost {amount} less (Fervent Champion)
    ReduceCostEquipTargetSourceEffect {
        #[serde(default)]
        amount: u32,
    },
    /// a static ability's effect: permanents of the filter enter tapped
    PermanentsEnterBattlefieldTappedEffect {
        filter: Filter,
    },
    /// a static ability's effect: players can't gain life
    CantGainLifeAllEffect,
    /// players can't gain life this turn (`CantGainLifeAllEffect(Duration.EndOfTurn)` as a spell's effect)
    CantGainLifeThisTurnEffect,
    /// return the source from its graveyard to the battlefield (`tapped`: Reassembling Skeleton)
    ReturnSourceFromGraveyardToBattlefieldEffect {
        #[serde(default)]
        tapped: bool,
    },
    /// put the source card (a dies trigger's graveyard card) on top of or under its owner's library
    PutOnLibrarySourceEffect {
        top: bool,
    },
    /// damage can't be prevented (the engine has no prevention yet, so nothing to do)
    DamageCantBePreventedEffect {
        duration: Duration,
    },
    /// exiles the targets
    ExileTargetEffect,
    /// the source gains the ability for as long as it's on the battlefield (`GainAbilitySourceEffect`, `Custom`)
    GainAbilityPermanentlyEffect {
        ability: Box<AbilityDef>,
    },
    /// activated abilities (not mana abilities) of sources with the source's chosen name can't be activated
    PithingNeedleEffect,
    /// Emry: you may cast the target (a nonland card in your graveyard) this turn
    MayCastTargetFromGraveyardThisTurnEffect,
    /// converge: exile the target nonland permanent if its mana value is at most the colors spent to cast this
    PrismaticEndingEffect,
    /// target player exiles a card from their graveyard
    RelicOfProgenitusEffect,
    /// `ExileAndGainLifeEqualPowerTargetEffect`: exile the target; its controller gains life equal to its power
    ExileAndGainLifeEqualPowerTargetEffect,
    /// `ExileThenReturnTargetEffect`: exile the targets, then return them under their owner's control
    ExileThenReturnTargetEffect,
    /// `ExileThenReturnTargetEffect(yourControl, …, PutCards)`: the same, under your control (`your_control`) or
    /// tapped
    ExileThenReturnUnderTargetEffect {
        #[serde(default)]
        your_control: bool,
        #[serde(default)]
        tapped: bool,
    },
    /// `PutCardFromHandOntoBattlefieldEffect`: you may (always asked) put a card of `filter` from your hand onto the
    /// battlefield, tapped or attacking
    PutCardFromHandOntoBattlefieldEffect {
        filter: Filter,
        #[serde(default)]
        tapped: bool,
        #[serde(default)]
        attacking: bool,
    },
    /// `MillCardsEachPlayerEffect`: each player of `who` ("any", "opponent"), from you in turn order, mills `amount`
    MillCardsEachPlayerEffect {
        amount: Amount,
        who: String,
    },
    /// `MillHalfLibraryTargetEffect`: the target players mill half their library, rounded down (or up)
    MillHalfLibraryTargetEffect {
        #[serde(default)]
        round_up: bool,
    },
    /// `PutOnTopOrBottomLibraryTargetEffect`: the first target's owner puts it on the top (`n`th from the top) or
    /// the bottom of their library, asked "Top" or "Bottom"
    PutOnTopOrBottomLibraryTargetEffect {
        #[serde(default = "one")]
        n: usize,
    },
    /// `SearchLibraryPutOnLibraryEffect`: search your library for cards of `filter`, shuffle, then put them on top
    SearchLibraryPutOnLibraryEffect {
        filter: Filter,
        #[serde(default = "one")]
        min: usize,
        #[serde(default = "one")]
        max: usize,
    },
    /// `ExileReturnBattlefieldNextEndStepTargetEffect`: exile the targets; at the beginning of the next end step
    /// return those cards to the battlefield under their owner's (or your) control, tapped or not
    ExileReturnBattlefieldNextEndStepTargetEffect {
        #[serde(default)]
        your_control: bool,
        #[serde(default)]
        tapped: bool,
    },
    /// the delayed half of `ExileReturnBattlefieldNextEndStepTargetEffect`: its fixed targets, cards still in exile,
    /// return to the battlefield (`ReturnToBattlefieldUnder…ControlTargetEffect`)
    ReturnToBattlefieldTargetEffect {
        #[serde(default)]
        your_control: bool,
        #[serde(default)]
        tapped: bool,
    },
    /// `ExileAndReturnSourceEffect(PutCards.BATTLEFIELD[_TAPPED])`: exile this permanent, then return it under its
    /// owner's (or your) control
    ExileAndReturnSourceEffect {
        #[serde(default)]
        your_control: bool,
        #[serde(default)]
        tapped: bool,
        /// `PutCards.BATTLEFIELD_TRANSFORMED`: it returns as its back face
        #[serde(default)]
        transformed: bool,
    },
    /// `ShuffleIntoLibraryTargetEffect`: the targets' owners shuffle them into their libraries (asked first when
    /// optional)
    ShuffleIntoLibraryTargetEffect {
        #[serde(default)]
        optional: bool,
    },
    /// `ExileAllEffect`: exile every permanent of the filter
    ExileAllEffect {
        filter: Filter,
    },
    /// `ExileAttachedEffect`: exile the permanent this is attached to
    ExileAttachedEffect,
    /// `DrawCardAllEffect`: each player of `who`, from you in turn order, draws `amount`
    DrawCardAllEffect {
        amount: Amount,
        who: String,
    },
    /// `ExileCardYouChooseTargetOpponentEffect`: the target opponent reveals their hand; you choose a card of the
    /// filter from it and exile it
    ExileCardYouChooseTargetOpponentEffect {
        filter: Filter,
    },
    /// `RevealLibraryPutIntoHandEffect`: reveal the top `amount`, put every card of the filter into your hand and
    /// the rest on the bottom of your library (in any order, or a random one) or into your graveyard
    RevealLibraryPutIntoHandEffect {
        amount: Amount,
        filter: Filter,
        #[serde(default)]
        graveyard: bool,
        #[serde(default)]
        any_order: bool,
    },
    /// `ShuffleIntoLibrarySourceEffect`: this object, while it's still the same one, is shuffled into its owner's
    /// library
    ShuffleIntoLibrarySourceEffect,
    /// `ShuffleYourGraveyardIntoLibraryEffect`
    ShuffleYourGraveyardIntoLibraryEffect,
    /// `CreateTokenAllEffect`: each player of `who`, from you in turn order, creates the tokens of `effect` (a
    /// `CreateTokenEffect`)
    CreateTokenAllEffect {
        who: String,
        effect: Box<EffectDef>,
    },
    /// `ExileCardsFromTopOfLibraryTargetEffect`: the target player exiles the top `amount` cards of their library
    ExileCardsFromTopOfLibraryTargetEffect {
        amount: Amount,
    },
    /// `ExileFromZoneTargetEffect(Zone.HAND, filter, amount, false)`: the target player exiles `amount` cards of the
    /// filter from their hand
    ExileFromHandTargetEffect {
        filter: Filter,
        #[serde(default = "one")]
        amount: usize,
    },
    /// rebound's upkeep trigger: its owner may cast the exiled card without paying its mana cost
    ReboundCastSpellFromExileEffect,
    /// returns the targets to their owners' hands
    ReturnToHandTargetEffect,
    /// return this to its owner's hand: from the battlefield, or (unless `battlefield_only`) the graveyard or exile
    /// it went to (Rancor)
    ReturnToHandSourceEffect {
        #[serde(default)]
        battlefield_only: bool,
    },
    TapTargetEffect,
    UntapTargetEffect,
    /// taps the source
    TapSourceEffect,
    /// counters the target spell unless its controller pays `cost`
    CounterUnlessPaysEffect {
        #[serde(default)]
        cost: Vec<CostDef>,
        /// `{X}`, worked out as it resolves, instead of a fixed cost
        #[serde(default)]
        generic: Option<Amount>,
        /// exiled instead of put into the graveyard
        #[serde(default)]
        exile: bool,
    },
    /// the controller draws `draw` cards, then discards `discard` (asked first when optional)
    DrawDiscardControllerEffect {
        draw: i32,
        discard: i32,
        #[serde(default)]
        optional: bool,
    },
    /// the controller discards `amount` cards
    DiscardControllerEffect {
        amount: Amount,
    },
    /// the target player reveals their hand and you choose a card of the filter; they discard it (Duress)
    DiscardCardYouChooseTargetEffect {
        filter: Filter,
    },
    /// exile the target until the source leaves the battlefield (Banishing Light)
    ExileUntilSourceLeavesEffect,
    /// the source explores `amount` times (`ExploreSourceEffect`)
    ExploreSourceEffect {
        amount: Amount,
    },
    /// the target creature explores
    ExploreTargetEffect,
    /// the target creature can't be blocked this turn
    CantBeBlockedTargetEffect {
        duration: Duration,
    },
    /// an Aura spell's effect: it's attached to its target as it enters
    AttachEffect,
    /// a static ability's effect: the permanent this is attached to gets +power/+toughness
    #[serde(alias = "BoostEquippedEffect")]
    BoostEnchantedEffect {
        power: Amount,
        toughness: Amount,
    },
    /// a static ability's effect: the permanent this is attached to can't attack or block
    CantAttackBlockAttachedEffect,
    /// each player (of `who`) discards `amount` cards they choose, all at once
    DiscardEachPlayerEffect {
        amount: Amount,
        who: String,
    },
    /// exile the target player's graveyard
    ExileGraveyardAllTargetPlayerEffect,
    /// exile all cards from all graveyards
    ExileGraveyardAllPlayersEffect,
    /// exile the target into this permanent's exile zone (Skyclave Apparition)
    ExileTargetForSourceEffect,
    /// the owner of the cards this exiled creates an X/X `token`, X their total mana value
    CreateXXTokenExiledEffect {
        token: TokenDef,
    },
    /// look at the target opponent's hand; you may exile a nonland card from it, which its owner may play for {2} more
    EliteSpellbinderEffect,
    /// the target opponent reveals their hand; you choose a card of the filter from it, exiled until this leaves
    KitesailFreebooterExileEffect {
        filter: Filter,
    },
    /// put this card from your hand onto the battlefield tapped and attacking (ninjutsu)
    NinjutsuEffect,
    /// the controller takes an extra turn after this one
    AddExtraTurnControllerEffect,
    /// the spell is exiled as it resolves, instead of going to the graveyard
    ExileSpellEffect,
    /// return the source to the battlefield; it's an enchantment, not a creature, from then on
    EnduringGlimmerReturnEffect,
    /// remove up to `amount` counters, of any kinds, from the target (Glissa Sunslayer)
    RemoveUpToAmountCountersEffect {
        amount: i32,
    },
    /// Fabled Passage: search your library for a card of the filter, shuffle, put it onto the battlefield tapped,
    /// then untap it if you control four or more lands
    FabledPassageSearchForLandEffect {
        filter: Filter,
    },
    /// its second half, done by the first
    FabledPassageUntapLandEffect,
    /// Scavenging Ooze: exile the target card from a graveyard; if it was a creature card, put a +1/+1 counter on
    /// this and you gain 1 life
    ScavengingOozeEffect,
    /// Koth, the Geomancer: 1 damage to each opponent; if the land that entered is a Mountain, add {R}
    KothTheGeomancerEffect,
    /// a static ability's effect: the turn this entered, its loyalty abilities can be activated at instant speed
    TheWanderingEmperorEffect,
    /// a static ability's effect: your Lizards, Mice, Otters and Raccoons deal 1 more damage
    ValleyFlamecallerEffect,
    /// a static ability's effect: noncombat damage your sources would deal to an opponent's creature is that many
    /// -1/-1 counters instead
    SoulScarMageDamageReplacementEffect,
    /// Kalastria Highborn: you gain 2 life, then the target player loses 2 life
    LoseGainEffect,
    /// Magmatic Hellkite: destroy the target land; its controller searches their library for a basic land card, puts
    /// it onto the battlefield tapped with a stun counter, then shuffles
    MagmaticHellkiteEffect,
    /// Floodpits Drowner: shuffle this and the target creature into their owners' libraries
    FloodpitsDrownerEffect,
    /// "choose a color": the permanents of `filter` gain protection from it until end of turn
    GainProtectionFromColorAllEffect {
        filter: Filter,
    },
    /// Elven Passage: search for a basic land, shuffle, put it onto the battlefield tapped; may behold an Elf to untap it
    ElvenPassageEffect,
    /// the controller gains life equal to the toughness of the permanent whose entering triggered the ability
    RighteousValkyrieEffect,
    /// `first` damage to the first target, then `second` damage to the second
    DamageTargetAndTargetEffect {
        first: i32,
        second: i32,
    },
    /// the source gains card types (until end of turn): a crewed Vehicle becomes an artifact creature
    AddCardTypeSourceEffect {
        types: Vec<CardType>,
        duration: Duration,
    },
    /// a characteristic-defining ability: the source's base power is a number (Lumbering Worldwagon)
    SetBasePowerSourceEffect {
        amount: Amount,
    },
    /// a static ability's effect: a card or token that would be put into a graveyard from anywhere is exiled
    /// instead (Rest in Peace)
    GraveyardFromAnywhereExileReplacementEffect,
    /// an opponent's card that would be put into a graveyard from anywhere is exiled instead
    LeylineOfTheVoidEffect,
    /// Valgavoth: a card you didn't control going to an opponent's graveyard is exiled with it instead
    ValgavothTerrorEaterReplacementEffect,
    /// Valgavoth: during your turn, the cards exiled with it may be played, spells for life
    ValgavothTerrorEaterPlayEffect,
    /// pay `cost` any number of times; when you do, that many +1/+1 counters on the source, then exile up to that
    /// many instants and sorceries (mana value 3 or less) from your graveyard and cast copies free
    BloodthirstyAdversaryEffect {
        cost: String,
    },
    /// the reflexive part: exile the target cards, then you may cast a copy of each without paying its mana cost
    BloodthirstyAdversaryCopyEffect,
    /// counter the target ability; its artifact, creature or planeswalker loses all abilities while the source stays
    TishanasTidebinderEffect,
    /// the target opponent reveals their hand; you exile a nonland card from it, yours to cast if the gift was promised
    CruelclawsHeistEffect,
    /// you may have the target creature gain reach until end of turn
    AcademicDisputeEffect,
    /// learn, with no cards outside the game: you may "wish" (for nothing), or else discard a card to draw one
    LearnEffect,
    /// exile the target spell; it becomes plotted (its owner may cast it free as a sorcery on a later turn)
    AvenInterrupterEffect,
    /// each other Angel you control enters with an additional +1/+1 counter for each Angel you already control
    GiadaFontOfHopeEntersBattlefieldEffect,
    /// a static ability's effect: each land is also the basic land type, with its mana ability (Urborg)
    AddBasicLandTypeAllLandsEffect {
        subtype: String,
    },
    /// a static ability's effect: each permanent the filter matches also has the subtype (Laughing Jasper Flint)
    AddCardSubtypeAllEffect {
        filter: Filter,
        subtype: String,
    },
    /// exile the top X cards of the target opponent's library (X: outlaws you control); you may cast spells from
    /// among them this turn, spending mana as though it were any type
    LaughingJasperFlintEffect,
    /// exile the top card of that player's library; you may cast it this turn
    RagavanNimblePilfererEffect,
    /// the defending player reveals the top card of their library; a land card goes to their hand
    GoblinGuideEffect,
    /// 1 damage to the first target (a player or planeswalker) and to the second (a creature that player controls);
    /// 3 each if a land entered under your control this turn (landfall)
    SearingBlazeEffect,
    /// your opponents can't cast noncreature spells this turn
    RangerCaptainOfEosEffect,
    /// the target land becomes an Island (losing its other land types and abilities) for as long as the source
    /// remains on the battlefield
    TideShaperEffect,
    /// you may put a creature card with mana value equal to the source's charge counters from your hand onto the
    /// battlefield
    AetherVialEffect,
    /// counter the target spell; it's exiled instead of put into its owner's graveyard
    CounterTargetExileEffect,
    /// the targets lose all abilities until end of turn (`LoseAllAbilitiesTargetEffect(Duration.EndOfTurn)`)
    LoseAllAbilitiesTargetEffect,
    /// a static ability's effect: the source doesn't untap during its controller's untap step
    DontUntapInControllersUntapStepSourceEffect,
    /// untap the source
    UntapSourceEffect,
    /// a regeneration shield on each target
    RegenerateTargetEffect,
    /// remove a time counter from the suspended source card; the last one sets off its `SuspendLastCounter` trigger
    SuspendRemoveTimeCounterEffect,
    /// its owner may cast the suspended source card without paying its mana cost; if they don't, it stays exiled
    SuspendPlayCardEffect,
    /// Unholy Annex: each opponent loses 2 life and you gain 2 life
    UnholyAnnexDrainEffect,
    /// the source becomes prepared: a copy of its prepare spell in exile, which its controller may cast
    BecomePreparedSourceEffect,
    /// if this spell was cast from a graveyard: exiled, then onto the battlefield transformed with a finality counter
    EsperOriginsEffect,
    /// reveal the top card of your library; a permanent card goes to your hand
    SummonEsperMaduinEffect,
    /// `ChooseACardNameEffect(LAND_NAME)`: the source's chosen name
    ChooseLandNameEffect,
    /// a static ability's effect: activated abilities of sources with the chosen name can't be activated unless
    /// they're mana abilities (Petrified Hamlet)
    PetrifiedHamletCostEffect,
    /// a static ability's effect: the spell costs {1} less for each permanent sacrificed for its `SacrificeXTargetCost`
    RottenmouthViperCostReductionEffect,
    /// for each blight counter on the source, each opponent loses 4 life unless they sacrifice a nonland permanent
    /// or discard a card
    RottenmouthViperEffect,
    /// Old-Growth Troll: it returns as an Aura enchanting a Forest you control, which has "{T}: Add {G}{G}" and
    /// "{1}, {T}, sacrifice this land: create a tapped `token`"
    OldGrowthTrollReturnEffect {
        token: serde_json::Value,
    },
    /// the target player mills `amount` cards
    MillCardsTargetEffect {
        amount: Amount,
    },
    /// the target becomes the token's kind of creature, its types and subtypes replaced, with its abilities too
    BecomesCreatureTargetEffect {
        token: TokenDef,
        duration: Duration,
    },
    /// if this isn't a creature, it becomes the token's kind of creature for good; still a land
    GreatHallOfTheBiblioplexEffect {
        token: TokenDef,
    },
    /// the source becomes the token's kind of creature (still its old types with `retain`), with its abilities too
    BecomesCreatureSourceEffect {
        token: TokenDef,
        #[serde(default)]
        retain: bool,
        duration: Duration,
    },
    /// mill four; you may put a permanent card from among them into your hand; then a Food if you control a
    /// Squirrel or returned one
    CacheGrabEffect {
        token: TokenDef,
    },
    /// return a creature card you choose from your graveyard to the battlefield with a finality counter
    ScavengersTalentEffect,
    /// put the target card from a graveyard onto the battlefield under your control
    ReturnFromGraveyardToBattlefieldTargetEffect {
        #[serde(default)]
        tapped: bool,
        /// under its owner's control (`ReturnToBattlefieldUnderOwnerControlTargetEffect`)
        #[serde(default)]
        owner: bool,
        /// it enters with these counters (`…WithCounterTargetEffect`)
        #[serde(default)]
        counters: Option<CounterDef>,
    },
    /// the target player discards `amount` cards of their choice
    DiscardTargetEffect {
        amount: Amount,
    },
    /// you get an emblem (abilities in the command zone), read like a token class
    GetEmblemEffect {
        emblem: TokenDef,
    },
    /// the target player sacrifices `amount` permanents of the filter
    SacrificeEffect {
        filter: Filter,
        amount: Amount,
    },
    /// each opponent sacrifices `amount` permanents of the filter (all chosen, then sacrificed together)
    SacrificeOpponentsEffect {
        filter: Filter,
        amount: Amount,
    },
    /// each player sacrifices `amount` permanents of the filter (all chosen, then sacrificed together)
    SacrificeAllEffect {
        filter: Filter,
        amount: Amount,
    },
    /// each opponent chooses a permanent they control of each permanent type and sacrifices the rest
    LilianaDreadhordeGeneralEffect,
    /// the target card in your graveyard gains flashback until end of turn, for its mana cost
    GainFlashbackTargetEffect,
    /// +1/+1 counters on the targets, as many on each as was divided to it
    DistributeCountersEffect,
    /// -1/-1 counters, divided as the targets were chosen (Splendid Agony)
    DistributeM1M1CountersEffect,
    /// damage divided as the targets were chosen (`DamageMultiEffect`)
    DamageMultiEffect,
    /// each nonland permanent you control is all colors
    LeylineOfTheGuildpactEffect,
    /// until end of turn, the source becomes an X/X green Hydra creature, still a land
    LairOfTheHydraEffect,
    /// lands you control are every basic land type
    BecomesAllBasicsControlledEffect,
    /// a static's rules text only (`InfoEffect`)
    InfoEffect,
    /// equip's effect: the source is attached to the target creature
    EquipEffect,
    /// an Aura's static: what it enchants is also the token's kind of creature, with its base power and toughness
    BecomesCreatureAttachedEffect {
        token: TokenDef,
    },
    /// a Siege defeated: exile it, then cast it transformed without paying its mana cost
    SiegeDefeatedEffect,
    /// a static: this can't attack or block unless the condition holds
    /// this attacks each combat if able (`AttacksIfAbleSourceEffect`, `AttacksEachCombatStaticAbility`), while the
    /// condition holds (`ConditionalRequirementEffect`)
    AttacksIfAbleSourceEffect {
        #[serde(default)]
        condition: Option<Condition>,
    },
    CantAttackBlockUnlessConditionSourceEffect {
        condition: Condition,
    },
    /// this can't be blocked by creatures of the filter (granted for the duration)
    CantBeBlockedByCreaturesSourceEffect {
        filter: Filter,
        duration: Duration,
    },
    /// 2 damage to the creature that entered; if it was a Dinosaur dealt damage, the source gets +2/+0 this turn
    MaraudingRaptorEffect,
    /// as this enters: tapped, and its controller chooses a color (its chosen-color mana abilities make that color)
    EntersTappedChooseColorEffect,
    /// a Case's "to solve": the source permanent becomes solved
    SolveEffect,
    /// creature cards in your graveyard may be cast from there until end of turn
    CaseOfTheUneatenFeastEffect,
    /// `MayCastTargetCardEffect` (one use): asked "Cast X?", then the target card is cast now, its costs paid (with
    /// mana of any type when `any_mana`)
    MayCastTargetCardEffect {
        #[serde(default)]
        any_mana: bool,
    },
    /// creatures with power less than this one's can't block it
    CantBeBlockedByCreaturesWithLessPowerEffect,
    /// damage from your sources to an opponent or a permanent of theirs is doubled
    TwinflameTyrantEffect,
    /// the controller mills `amount` cards
    MillCardsControllerEffect {
        amount: Amount,
    },
    /// a static effect on a spell: it can't be countered
    CantBeCounteredSourceEffect,
    /// `SearchLibraryPutIn…Effect`: the controller (or the target's controller) searches their library for cards of
    /// the filter, puts them `to`, then shuffles; asked first when optional (always, for the target's controller)
    SearchLibraryEffect {
        filter: Filter,
        #[serde(default = "one")]
        min: usize,
        #[serde(default = "one")]
        max: usize,
        to: PutCards,
        #[serde(default)]
        optional: bool,
        #[serde(default)]
        target_controller: bool,
    },
    /// the source deals `amount` damage to its controller
    DamageControllerEffect {
        amount: Amount,
    },
    /// the target players draw `amount` cards
    DrawCardTargetEffect {
        amount: Amount,
    },
    /// the controller scries `amount`
    ScryEffect {
        amount: Amount,
    },
    /// the controller surveils `amount`
    /// static: nonland permanent cards in graveyards and libraries can't enter the battlefield
    WeatheredRunestoneEffect,
    /// static: players can't cast spells from graveyards or libraries
    WeatheredRunestoneEffect2,
    /// as this enters, choose a permanent of the filter (not targeted)
    ChooseCreatureEffect {
        filter: Filter,
    },
    /// the creature chosen as the source entered gains the ability until end of turn
    ChosenCreatureGainsAbilityEffect {
        ability: Box<AbilityDef>,
    },
    /// evolve's +1/+1 counter, if the creature that entered still has greater power or toughness
    EvolveEffect,
    /// a regeneration shield on the source until end of turn
    RegenerateSourceEffect,
    /// Greasefang: the target Vehicle card returns with haste, back to hand at the next end step
    GreasefangOkibaBossEffect,
    /// pay `cost` any number of times, then `effects` with X the times paid (a reflexive trigger with that rule)
    DoIfAnyNumberCostPaid {
        cost: Vec<CostDef>,
        effects: Vec<EffectDef>,
        #[serde(default)]
        reflexive: Option<String>,
    },
    /// the source (and/or the target's permanents: `target`, without `source`) gains the chosen one of `abilities`
    /// (named by `names`, their rule texts) until end of turn
    GainsChoiceOfAbilitiesEffect {
        abilities: Vec<AbilityDef>,
        names: Vec<String>,
        #[serde(default = "yes")]
        source: bool,
        #[serde(default)]
        target: bool,
    },
    /// creates one token with power and toughness `amount` (`new SharkToken(xValue)`)
    CreateXXTokenEffect {
        token: TokenDef,
        amount: Amount,
    },
    /// the source gets +X/+0 until end of turn (X: creature cards in your graveyard), then may fight the target
    KraulHarpoonerEffect,
    /// surveil 1; a card put into the graveyard with mana value up to the life you gained this turn goes to your hand
    EnlightenedConfidantEffect,
    SurveilEffect {
        amount: Amount,
    },
    /// looks at the top `look` cards (or `look_count` of them), puts up to (or exactly) `pick` of `filter` where
    /// `picked` says and the rest where `looked` says; `optional` asks first
    LookLibraryAndPickControllerEffect {
        #[serde(default)]
        look: i32,
        #[serde(default)]
        look_count: Option<String>,
        pick: i32,
        #[serde(default)]
        filter: Filter,
        picked: PutCards,
        looked: PutCards,
        #[serde(default)]
        up_to: bool,
        #[serde(default)]
        optional: bool,
        /// pick up to the X paid in place of `pick` (Kayla's Reconstruction)
        #[serde(default)]
        pick_x: bool,
    },
    /// a static ability's effect: the permanents of `filter` have `ability`
    /// a static ability's effect: the permanent this Aura enchants has the ability
    GainAbilityAttachedEffect {
        ability: Box<AbilityDef>,
    },
    /// a Role token attached to each target (`CreateRoleAttachedTargetEffect`)
    CreateRoleAttachedTargetEffect {
        token: TokenDef,
    },
    GainAbilityAllEffect {
        ability: Box<AbilityDef>,
        duration: Duration,
        filter: Filter,
        #[serde(default)]
        exclude_source: bool,
    },
}

/// The spell ability of an instant or sorcery: what it does when it resolves. A modal spell ("choose one") has
/// `modes` instead, the first being the spell ability's own effects and targets.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct SpellDef {
    /// the spell ability's rule text, as Java prints it (lower case: auto-targeting reads it)
    #[serde(default)]
    pub rule: String,
    #[serde(default)]
    pub effects: Vec<EffectDef>,
    #[serde(default)]
    pub targets: Vec<TargetDef>,
    #[serde(default)]
    pub modes: Vec<ModeDef>,
    /// how many modes are chosen ("choose two": 2 and 2)
    #[serde(default = "one")]
    pub min_modes: usize,
    #[serde(default = "one")]
    pub max_modes: usize,
    /// costs on top of the mana cost ("as an additional cost to cast this spell"), paid after the mana
    #[serde(default)]
    pub costs: Vec<CostDef>,
    /// the targets chosen instead when cast unkicked and kicked (`ConditionalTargetAdjuster`); `targets` is then
    /// what either could target
    #[serde(default)]
    pub kicked_targets: Option<(Vec<TargetDef>, Vec<TargetDef>)>,
    /// the same, without and with the gift promised
    #[serde(default)]
    pub gift_targets: Option<(Vec<TargetDef>, Vec<TargetDef>)>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct ModeDef {
    #[serde(default)]
    pub effects: Vec<EffectDef>,
    #[serde(default)]
    pub targets: Vec<TargetDef>,
    /// a spree mode's additional mana cost
    #[serde(default)]
    pub cost: Option<String>,
}

impl SpellDef {
    /// the effects and targets of mode `i` (of the spell itself when it has no modes)
    pub fn mode(&self, i: usize) -> (&[EffectDef], &[TargetDef]) {
        match self.modes.get(i) {
            Some(m) => (&m.effects, &m.targets),
            None => (&self.effects, &self.targets),
        }
    }
}

/// Abilities of a card, named after the Java ability classes, in the order Java lists them (decision logs name an
/// ability by its place in that list). `rule` is the rule text as Java prints it, `{this}` and all.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(tag = "class", deny_unknown_fields)]
pub enum AbilityDef {
    /// casting the card (every nonland card has one)
    SpellAbility,
    /// a Room entering unlocks the door of the half cast (`RoomCard`'s `EntersBattlefieldAbility`)
    RoomEnterUnlockAbility,
    /// a Room's shared type line: only its unlocked doors' names, costs and abilities (`RoomAbility`)
    RoomAbility {
        rule: String,
    },
    /// the special action to unlock a locked door, paying its cost, as a sorcery (`RoomUnlockAbility`)
    RoomUnlockAbility {
        left: bool,
        cost: String,
    },
    /// playing the card (every land has one)
    PlayLandAbility,
    SimpleStaticAbility {
        effects: Vec<EffectDef>,
        rule: String,
    },
    /// a triggered ability: what it triggers on, then what it does (Java's many `…TriggeredAbility` classes, each
    /// one `Trigger`)
    TriggeredAbility {
        on: Trigger,
        effects: Vec<EffectDef>,
        #[serde(default)]
        targets: Vec<TargetDef>,
        #[serde(default)]
        optional: bool,
        /// an intervening "if", checked as it triggers and as it resolves
        #[serde(default)]
        condition: Option<Condition>,
        /// a condition checked only as it triggers (`withTriggerCondition`)
        #[serde(default)]
        trigger_condition: Option<Condition>,
        /// the effects act on the object or player the event was about (`SetTargetPointer`)
        #[serde(default)]
        set_target_pointer: bool,
        /// the effects act on the controller of the object the event was about (`SetTargetPointer.PLAYER`)
        #[serde(default)]
        set_target_pointer_player: bool,
        /// it triggers at most this many times each turn (`setTriggersLimitEachTurn`)
        #[serde(default)]
        limit_per_turn: Option<u32>,
        /// a modal ability ("choose one"): its other modes, mode 0 being its own effects and targets
        #[serde(default)]
        modes: Vec<ModeDef>,
        /// its target is "… that player controls" (`ThatPlayerControlsTargetAdjuster`): on the stack its text names
        /// the player, "(controlled by B)"
        #[serde(default)]
        that_player_controls: bool,
        /// the target's name there when the rule doesn't say "that player controls" (its filter's message)
        #[serde(default)]
        that_player_name: Option<String>,
        rule: String,
    },
    EntersBattlefieldTappedAbility {
        rule: String,
    },
    /// a Saga: a lore counter as it enters and after your draw step; sacrificed after chapter `max`
    SagaAbility {
        max: u32,
        rule: String,
    },
    /// a replacement effect applied as it enters (shock lands)
    AsEntersBattlefieldAbility {
        effects: Vec<EffectDef>,
        rule: String,
    },
    /// cast from the graveyard for this cost, then exiled
    FlashbackAbility {
        cost: String,
        /// a cost that isn't mana (Lava Dart's "Sacrifice a Mountain")
        #[serde(default)]
        costs: Vec<CostDef>,
        rule: String,
    },
    /// cast from the graveyard for this cost, which tapping a creature reduces by its power, then exiled
    HarmonizeAbility {
        cost: String,
        rule: String,
    },
    /// cast from the graveyard for `cost` and exiling `exile` other cards from it
    EscapeAbility {
        cost: String,
        exile: usize,
        rule: String,
    },
    /// it enters with this many +1/+1 counters if it escaped
    EscapesWithAbility {
        counters: i32,
        rule: String,
    },
    /// cast from hand for `cost` instead, doing `effects` in place of the spell's (a second spell ability)
    CleaveAbility {
        cost: String,
        effects: Vec<EffectDef>,
        /// the cleaved spell's own targets, when they aren't the spell's (the words in brackets restrict them)
        #[serde(default)]
        targets: Option<Vec<TargetDef>>,
        rule: String,
    },
    /// overload: cast for `cost` instead, with no targets and `effects`, the spell's applied to each permanent its
    /// target could have chosen (a second spell ability, `OverloadAbility`)
    OverloadAbility {
        cost: String,
        effects: Vec<EffectDef>,
        rule: String,
    },
    /// cast for `mana` and `cost` instead: it gains haste and "when this creature dies, draw a card", and it's
    /// sacrificed at the beginning of the next end step (a second spell ability)
    /// gift: you may promise an opponent `name` as you cast it (an instant or sorcery: they get it as it resolves,
    /// `token` or else a card drawn)
    GiftAbility {
        name: String,
        #[serde(default)]
        token: Option<TokenDef>,
        rule: String,
    },
    /// spree: the spell's modes each cost more, and one or more are chosen (the modes carry the costs)
    SpreeAbility {
        rule: String,
    },
    /// tiered: choose one of the spell's modes, each with its additional cost
    TieredAbility {
        rule: String,
    },
    /// you may pay `cost` more as you cast it; if you do, when it enters, create a 1/1 token copy of it
    OffspringAbility {
        cost: String,
        rule: String,
    },
    BlitzAbility {
        mana: String,
        #[serde(default)]
        cost: Vec<CostDef>,
        rule: String,
    },
    /// whenever you tap a permanent of the filter for mana, add one more mana of `color` (a mana ability that
    /// triggers, resolved at once)
    TapForManaAllTriggeredManaAbility {
        filter: Filter,
        color: String,
        rule: String,
    },
    /// the blitz ability's "as it enters" part (Java lists it on its own)
    BlitzEntersAbility {
        rule: String,
    },
    /// cast from hand for `cost` instead if an opponent lost life this turn (a second spell ability)
    SpectacleAbility {
        cost: String,
        rule: String,
    },
    /// cast from hand for `cost` instead, at sorcery speed; exiled at the next end step, castable from exile on a
    /// later turn
    WarpAbility {
        cost: String,
        rule: String,
    },
    /// an optional additional cost asked for as the spell is cast
    /// equip: attach this to target creature you control, as a sorcery; `colorless_discount` less when it targets a
    /// colorless creature (Ghostfire Blade)
    EquipAbility {
        effects: Vec<EffectDef>,
        cost: Vec<CostDef>,
        targets: Vec<TargetDef>,
        #[serde(default)]
        colorless_discount: u32,
        #[serde(default)]
        rule: String,
    },
    /// cast by paying `costs` instead of the mana cost while the condition holds (`AlternativeCostSourceAbility`:
    /// Force of Negation)
    AlternativeCostSourceAbility {
        costs: Vec<CostDef>,
        #[serde(default)]
        condition: Option<Condition>,
        #[serde(default)]
        rule: String,
    },
    /// evoke: cast for this cost instead (an alternative cost); its sacrifice trigger is the next ability
    EvokeAbility {
        cost: String,
        /// the costs that aren't mana (Solitude's "exile a white card from your hand")
        #[serde(default)]
        costs: Vec<CostDef>,
        #[serde(default)]
        rule: String,
        /// the alternative cost's keyword: empty for evoke, "Dash" for dash (gains haste, returned to its owner's
        /// hand at the beginning of the next end step)
        #[serde(default)]
        keyword: String,
    },
    /// replicate: pay this cost any number of times as you cast it; a cast trigger copies it that many times
    ReplicateAbility {
        cost: String,
        #[serde(default)]
        rule: String,
    },
    KickerAbility {
        cost: String,
        rule: String,
    },
    /// "as an additional cost, you may blight N": N -1/-1 counters on a creature you control
    BlightAbility {
        amount: i32,
        rule: String,
    },
    /// enters with its effects applied (a replacement effect), if the condition holds
    EntersBattlefieldAbility {
        effects: Vec<EffectDef>,
        #[serde(default)]
        condition: Option<Condition>,
        rule: String,
    },
    /// bargain: as it's cast, you may sacrifice an artifact, enchantment, or token (the spell is then kicked);
    /// `discount`: it costs that much less if bargained (Ice Out)
    BargainAbility {
        #[serde(default)]
        discount: u32,
        #[serde(default)]
        rule: String,
    },
    /// casualty `n`: as it's cast, you may sacrifice a creature with power `n` or greater; when you do, copy the spell
    CasualtyAbility {
        n: i32,
        rule: String,
    },
    /// teamwork `n`: as it's cast, you may tap creatures you control with total power `n` or more (the spell is then
    /// kicked)
    TeamworkAbility {
        n: i32,
        rule: String,
    },
    /// `ExileCardsFromHandAdjuster`: as it's cast, you may exile any number of cards of the filter from your hand; it
    /// costs less for each (Java takes {4} off per card: it doubles the reduction twice)
    ExileCardsFromHandAdjusterAbility {
        filter: Filter,
        #[serde(default)]
        rule: String,
    },
    /// foretell `cost`: during your turn, pay {2} and exile it from your hand face down; cast it on a later turn for
    /// `cost`
    ForetellAbility {
        cost: String,
        #[serde(default)]
        rule: String,
    },
    /// plot `cost`: as a sorcery, pay it and exile the card from your hand; cast it free as a sorcery on a later turn
    PlotAbility {
        cost: String,
        #[serde(default)]
        rule: String,
    },
    /// suspend, a special action: from hand, pay `cost` and exile it with `time` time counters (as its spell could
    /// be cast); its upkeep and last-counter triggers are the next two abilities
    SuspendAbility {
        time: i32,
        cost: String,
        #[serde(default)]
        rule: String,
    },
    /// "(Gain the next level as a sorcery to add its ability.)"
    ClassReminderAbility,
    /// "If this card is in your opening hand, you may begin the game with it on the battlefield." (before the game)
    LeylineAbility,
    /// "You may reveal this card from your opening hand. If you do, at the beginning of your first upkeep, look at
    /// the top four cards …" (Devourer of Destiny's `ChancellorAbility`; the host asks it before the first turn)
    ChancellorAbility,
    /// it can't attack (given: `CantAttackTargetEffect`)
    CantAttackAbility,
    /// its activated abilities can't be activated (given: Edifice of Authority)
    CantActivateAbilitiesAbility,
    /// "If this card is in your opening hand and you're not the starting player, you may begin the game with it on
    /// the battlefield with a luck counter on it. If you do, exile a card from your hand." (the host asks it)
    GemstoneCavernsAbility,
    /// a Class's next level: activated as a sorcery when the Class is the level below
    ClassLevelAbility {
        level: u32,
        effects: Vec<EffectDef>,
        cost: Vec<CostDef>,
        #[serde(default)]
        targets: Vec<TargetDef>,
        rule: String,
    },
    ActivateAsSorceryActivatedAbility {
        effects: Vec<EffectDef>,
        cost: Vec<CostDef>,
        #[serde(default)]
        targets: Vec<TargetDef>,
        rule: String,
    },
    /// a planeswalker's `+N:`/`-N:` ability: sorcery speed, one loyalty ability per permanent each turn; its cost
    /// is a `PayLoyaltyCost`
    LoyaltyAbility {
        effects: Vec<EffectDef>,
        cost: Vec<CostDef>,
        #[serde(default)]
        targets: Vec<TargetDef>,
        rule: String,
    },
    /// an activated ability that works at most `limit` times each turn, and only while the condition holds
    LimitedTimesPerTurnActivatedAbility {
        effects: Vec<EffectDef>,
        cost: Vec<CostDef>,
        #[serde(default)]
        targets: Vec<TargetDef>,
        rule: String,
        limit: u32,
        #[serde(default)]
        condition: Option<Condition>,
    },
    /// an activated ability from hand: pay, discard this, draw (`CyclingAbility`)
    CyclingAbility {
        effects: Vec<EffectDef>,
        cost: Vec<CostDef>,
        #[serde(default)]
        targets: Vec<TargetDef>,
        rule: String,
    },
    /// an activated ability from hand with a discard-this cost (`ChannelAbility`); with the legendary discount, {1}
    /// less for each legendary creature you control (`LegendaryCreatureCostAdjuster`)
    /// Ninjutsu: from hand, `cost` and return an unblocked attacker you control: this enters tapped and attacking
    NinjutsuAbility {
        effects: Vec<EffectDef>,
        cost: Vec<CostDef>,
        rule: String,
    },
    ChannelAbility {
        effects: Vec<EffectDef>,
        cost: Vec<CostDef>,
        #[serde(default)]
        targets: Vec<TargetDef>,
        rule: String,
        #[serde(default)]
        legendary_discount: bool,
    },
    SimpleActivatedAbility {
        effects: Vec<EffectDef>,
        cost: Vec<CostDef>,
        #[serde(default)]
        targets: Vec<TargetDef>,
        rule: String,
        /// the least X it can be activated with (`VariableManaCost.setMinX`)
        #[serde(default)]
        min_x: i32,
    },
    /// an activated ability of a card in your graveyard (`ActivateIfConditionActivatedAbility` with
    /// `Zone.GRAVEYARD`), only while the condition holds, at sorcery speed when `sorcery`
    GraveyardActivatedAbility {
        effects: Vec<EffectDef>,
        cost: Vec<CostDef>,
        #[serde(default)]
        targets: Vec<TargetDef>,
        rule: String,
        #[serde(default)]
        sorcery: bool,
        #[serde(default)]
        condition: Option<Condition>,
    },
    /// power-up: once each game, and for its mana cost less the turn this entered
    PowerUpAbility {
        effects: Vec<EffectDef>,
        cost: Vec<CostDef>,
        #[serde(default)]
        targets: Vec<TargetDef>,
        rule: String,
    },
    /// exhaust: an activated ability each object can activate only once (`ExhaustAbility`)
    ExhaustAbility {
        effects: Vec<EffectDef>,
        cost: Vec<CostDef>,
        #[serde(default)]
        targets: Vec<TargetDef>,
        #[serde(default)]
        sorcery: bool,
        rule: String,
    },
    /// read ahead: a Saga's chapters skipped as it enters don't trigger (`ReadAheadAbility`)
    ReadAheadAbility,
    /// skulk: can't be blocked by creatures with greater power
    SkulkAbility,
    /// intimidate: can be blocked only by artifact creatures and creatures that share a color with it
    IntimidateAbility,
    /// decayed: can't block (its attack trigger is listed on its own)
    DecayedAbility,
    /// this spell can't be countered
    CantBeCounteredSourceAbility,
    FlyingAbility,
    ReachAbility,
    /// it blocks this turn if able (`BlocksIfAbleTargetEffect`, granted until end of turn)
    BlocksIfAbleAbility,
    /// it can attack as though it didn't have defender (`CanAttackAsThoughItDidntHaveDefenderSourceEffect`)
    CanAttackAsThoughNoDefenderAbility,
    /// it must be blocked this turn if able (`MustBeBlockedByAtLeastOneTargetEffect`, granted until end of turn)
    MustBeBlockedIfAbleAbility,
    HasteAbility,
    VigilanceAbility,
    TrampleAbility,
    FirstStrikeAbility,
    DoubleStrikeAbility,
    LifelinkAbility,
    DeathtouchAbility,
    DefenderAbility,
    /// this can block only creatures with flying
    CanBlockOnlyFlyingAbility,
    /// an Aura: what it can enchant is its spell's target
    EnchantAbility,
    /// when this becomes the target of a spell or ability an opponent controls, counter it unless that player pays
    /// (its effect, a `CounterUnlessPaysEffect`)
    WardAbility {
        effects: Vec<EffectDef>,
        rule: String,
    },
    /// this can't block
    CantBlockAbility,
    /// this can't be blocked by creatures with power 2 or less
    DauntAbility,
    /// this can't be blocked as long as the defending player controls an Island
    IslandwalkAbility,
    /// umbra armor: if the enchanted creature would be destroyed, its damage is removed and this Aura is destroyed
    UmbraArmorAbility,
    /// an adventurer card's adventure spell (its first ability; the card's own spell is the second)
    AdventureCardSpellAbility,
    /// a preparation card's prepare spell (its `prepare`), which only a prepared copy casts
    PrepareCardSpellAbility,
    /// it enters prepared
    EntersPreparedAbility {
        #[serde(default)]
        rule: String,
    },
    OmenCardSpellAbility,
    /// Delve and Convoke: Java counts the graveyard's cards, or the untapped creatures, as mana when it works out
    /// whether the spell can be cast; paying with them is a special action ("special" while paying,
    /// `rules::special_mana_payment`) the shadow games' players never take
    DelveAbility,
    ConvokeAbility,
    /// improvise: each untapped artifact you control can pay {1}
    ImproviseAbility,
    /// sunburst: enters with a charge counter (a +1/+1 counter, if a creature) for each color of mana spent to cast it
    SunburstAbility,
    /// if a player casts no spells during their own turn, it becomes night next turn (the front of a werewolf)
    DayboundAbility,
    /// if a player casts at least two spells during their own turn, it becomes day next turn (its back)
    NightboundAbility,
    /// "you may exert this as it attacks"; its `BecomesExertSourceTriggeredAbility` is the next ability
    ExertAbility,
    /// splice onto `subtype` `cost`: as you cast a spell of that subtype, you may reveal this card from your hand and
    /// pay the cost; its effects are added to that spell
    SpliceAbility {
        subtype: String,
        cost: String,
        #[serde(default)]
        rule: String,
    },
    /// this can't be blocked (`CantBeBlockedSourceAbility`, or given by `CantBeBlockedTargetEffect`)
    CantBeBlockedSourceAbility,
    /// protection from objects of the filter (`ProtectionAbility`)
    ProtectionAbility {
        filter: Filter,
    },
    /// creatures of the filter can't block this (given by `CantBeBlockedByCreaturesSourceEffect`)
    CantBeBlockedByAbility {
        filter: Filter,
    },
    MenaceAbility,
    FlashAbility,
    /// rebound: cast from hand, it's exiled as it resolves, and its owner may cast it free at their next upkeep
    ReboundAbility,
    /// devoid: no colors (the card's colors are set empty; this is its keyword)
    DevoidAbility,
    /// toxic N: combat damage to a player also gives them N poison counters
    ToxicAbility {
        amount: i32,
        #[serde(default)]
        rule: String,
    },
    /// changeling: the card is every creature type (`CardDef::all_creature_types`)
    ChangelingAbility,
    HexproofAbility,
    /// protection from a color, as an effect grants it (Brave the Elements)
    ProtectionFromColorAbility {
        c: Color,
    },
    IndestructibleAbility,
    /// {T}: Add {W}. The basic land types' mana abilities are these too.
    WhiteManaAbility {
        /// what else activating it does (a pain land's damage)
        #[serde(default)]
        effects: Vec<EffectDef>,
        /// costs besides {T} (Ramunap Ruins' life)
        #[serde(default)]
        cost: Vec<CostDef>,
    },
    BlueManaAbility {
        #[serde(default)]
        effects: Vec<EffectDef>,
        #[serde(default)]
        cost: Vec<CostDef>,
    },
    BlackManaAbility {
        #[serde(default)]
        effects: Vec<EffectDef>,
        #[serde(default)]
        cost: Vec<CostDef>,
    },
    RedManaAbility {
        #[serde(default)]
        effects: Vec<EffectDef>,
        #[serde(default)]
        cost: Vec<CostDef>,
    },
    GreenManaAbility {
        #[serde(default)]
        effects: Vec<EffectDef>,
        #[serde(default)]
        cost: Vec<CostDef>,
    },
    ColorlessManaAbility {
        #[serde(default)]
        effects: Vec<EffectDef>,
        #[serde(default)]
        cost: Vec<CostDef>,
        /// it doesn't tap its source (an Eldrazi Scion's "Sacrifice this creature: Add {C}.")
        #[serde(default)]
        no_tap: bool,
        /// how much {C} (Weaver of Currents' two)
        #[serde(default = "one_u32")]
        amount: u32,
        /// {C}{C} instead while a card is exiled with the source (Ugin's Labyrinth)
        #[serde(default)]
        two_if_exiled: bool,
        /// one mana of any color instead while the source has a counter of this kind (Gemstone Caverns' luck)
        #[serde(default)]
        any_color_with: Option<String>,
        #[serde(default)]
        rule: String,
    },
    /// `{T}: Add` `amount` mana of one color (`SimpleManaAbility` with `Mana.GreenMana(2)`)
    ColoredManaAbility {
        color: Color,
        amount: u32,
        /// its costs besides tapping (or instead of it, `no_tap`)
        #[serde(default)]
        cost: Vec<CostDef>,
        /// it doesn't tap its source (Heritage Druid's "Tap three untapped Elves you control")
        #[serde(default)]
        no_tap: bool,
        /// "if any of that mana is spent on a creature spell, it gains haste until end of turn" (Arena of Glory)
        #[serde(default)]
        creature_spell_haste: bool,
        /// how many times `amount` it counts for while checking what's playable (Skirk Prospector: one per Goblin it
        /// could sacrifice; `BasicManaEffect`'s `netAmount`)
        #[serde(default)]
        net: Option<Amount>,
        #[serde(default)]
        rule: String,
    },
    /// `{T}: Add {otherwise}. If <condition>, add {color} instead` (River of Tears)
    ConditionalColorManaAbility {
        color: Color,
        otherwise: Color,
        condition: Condition,
        /// how much of `color` (Leafkin Druid's {G}{G}), and of `otherwise`
        #[serde(default = "one_u32")]
        amount: u32,
        #[serde(default = "one_u32")]
        otherwise_amount: u32,
        #[serde(default)]
        rule: String,
    },
    /// `{T}: Add` `amount` mana of `color`, counted as it's activated (`DynamicManaAbility`: Elvish Archdruid)
    DynamicManaAbility {
        /// none: `{C}` (Powerstone Shard)
        #[serde(default)]
        color: Option<Color>,
        amount: Amount,
        /// its costs besides tapping (Cabal Stronghold's {3}), or instead of it (`no_tap`)
        #[serde(default)]
        cost: Vec<CostDef>,
        #[serde(default)]
        no_tap: bool,
        #[serde(default)]
        rule: String,
    },
    /// `{T}[, other costs]: Add one mana of any color` (a Treasure's, with a sacrifice)
    AnyColorManaAbility {
        /// how much of the chosen color (a Lotus's three)
        #[serde(default = "one_u32")]
        amount: u32,
        #[serde(default)]
        effects: Vec<EffectDef>,
        #[serde(default)]
        cost: Vec<CostDef>,
        /// it doesn't tap its source (Relic of Legends' "Tap an untapped legendary creature you control")
        #[serde(default)]
        no_tap: bool,
        #[serde(default)]
        rule: String,
    },
    /// `{T}: Add {color}. Activate only if …` (the Verges)
    ActivateIfConditionManaAbility {
        color: Color,
        condition: Condition,
        rule: String,
    },
    /// `{T}[, other costs]: Add one mana of any color. Activate only if …` (Spire of Industry)
    ActivateIfConditionAnyColorManaAbility {
        condition: Condition,
        #[serde(default)]
        cost: Vec<CostDef>,
        rule: String,
    },
    /// `{T}: Add {color}` (or `amount` of any one color, with no color). Spend this mana only to cast a spell of the
    /// filter (`ConditionalSpellManaBuilder`)
    ConditionalManaAbility {
        #[serde(default)]
        color: Option<Color>,
        #[serde(default = "one_u32")]
        amount: u32,
        /// more than one of any color: in any combination of colors (`getMultiAmount`), not all of one
        #[serde(default)]
        combination: bool,
        filter: Filter,
        /// and of the source's chosen creature type (Unclaimed Territory)
        #[serde(default)]
        chosen_type: bool,
        /// the spell it's spent on can't be countered (Cavern of Souls)
        #[serde(default)]
        cant_counter: bool,
        /// its costs besides tapping (Great Hall's 1 life)
        #[serde(default)]
        cost: Vec<CostDef>,
        /// spent only to activate abilities of lands (Sunken Citadel), not on spells
        #[serde(default)]
        land_abilities: bool,
        /// spent on activated abilities of creatures too (Castle Garenbrig)
        #[serde(default)]
        creature_abilities: bool,
        /// it makes colorless mana, `amount` of it (Eldrazi Temple)
        #[serde(default)]
        colorless: bool,
        /// spent on activated abilities of permanents of `filter` too (Eldrazi Temple)
        #[serde(default)]
        filter_abilities: bool,
        #[serde(default)]
        rule: String,
    },
    /// `{T}: Add one mana of any color in your commander's color identity` (Command Tower). Duels have no
    /// commander: Java still lets it be activated, and it makes nothing.
    CommanderColorIdentityManaAbility {
        #[serde(default)]
        rule: String,
    },
    /// `{T}: Add one mana of any color among` permanents of `filter` you control (Mox Amber; none: no mana)
    AnyColorPermanentTypesManaAbility {
        filter: Filter,
        #[serde(default)]
        rule: String,
    },
    /// Nykthos: `{2}, {T}`: choose a color, add mana of it equal to your devotion to it
    NykthosShrineToNyxManaAbility {
        cost: Vec<CostDef>,
        #[serde(default)]
        rule: String,
    },
    /// `{T}: Add` `amount` mana of the color chosen as it entered (Sunken Citadel; `land_abilities`: only for
    /// abilities of lands). Becomes a `ColoredManaAbility` or a `ConditionalManaAbility` once the color is chosen.
    ChosenColorManaAbility {
        amount: u32,
        #[serde(default)]
        land_abilities: bool,
        #[serde(default)]
        rule: String,
    },
}

/// An activated ability's parts: rule text, effects, targets, costs, sorcery speed.
pub type Activated<'a> = (&'a str, &'a [EffectDef], &'a [TargetDef], &'a [CostDef], bool);

impl AbilityDef {
    /// a triggered ability's rule text, effects, targets and whether it's a "may" ability
    pub fn triggered(&self) -> Option<(&str, &[EffectDef], &[TargetDef], bool)> {
        match self {
            AbilityDef::TriggeredAbility { effects, targets, optional, rule, .. } => {
                Some((rule, effects, targets, *optional))
            }
            AbilityDef::WardAbility { effects, rule } => Some((rule, effects, &[], false)),
            _ => None,
        }
    }

    /// what a triggered ability triggers on
    pub fn trigger(&self) -> Option<&Trigger> {
        match self {
            AbilityDef::TriggeredAbility { on, .. } => Some(on),
            _ => None,
        }
    }

    /// a triggered ability's intervening "if", checked when it triggers and again when it resolves
    pub fn trigger_condition(&self) -> Option<&Condition> {
        match self {
            AbilityDef::TriggeredAbility { trigger_condition, .. } => trigger_condition.as_ref(),
            _ => None,
        }
    }

    pub fn intervening_if(&self) -> Option<&Condition> {
        match self {
            AbilityDef::TriggeredAbility { condition, .. } => condition.as_ref(),
            _ => None,
        }
    }

    /// an activated ability's rule text, effects, targets, costs and whether it's sorcery speed
    pub fn activated(&self) -> Option<Activated<'_>> {
        match self {
            AbilityDef::ActivateAsSorceryActivatedAbility { effects, cost, targets, rule }
            | AbilityDef::ClassLevelAbility { effects, cost, targets, rule, .. } => {
                Some((rule, effects, targets, cost, true))
            }
            AbilityDef::SimpleActivatedAbility { effects, cost, targets, rule, .. }
            | AbilityDef::PowerUpAbility { effects, cost, targets, rule } => {
                Some((rule, effects, targets, cost, false))
            }
            AbilityDef::ExhaustAbility { effects, cost, targets, rule, sorcery } => {
                Some((rule, effects, targets, cost, *sorcery))
            }
            AbilityDef::LoyaltyAbility { effects, cost, targets, rule } => Some((rule, effects, targets, cost, true)),
            AbilityDef::EquipAbility { effects, cost, targets, rule, .. } => Some((rule, effects, targets, cost, true)),
            AbilityDef::CyclingAbility { effects, cost, targets, rule } => Some((rule, effects, targets, cost, false)),
            AbilityDef::LimitedTimesPerTurnActivatedAbility { effects, cost, targets, rule, .. } => {
                Some((rule, effects, targets, cost, false))
            }
            AbilityDef::ChannelAbility { effects, cost, targets, rule, .. } => {
                Some((rule, effects, targets, cost, false))
            }
            AbilityDef::NinjutsuAbility { effects, cost, rule } => Some((rule, effects, &[], cost, false)),
            AbilityDef::GraveyardActivatedAbility { effects, cost, targets, rule, sorcery, .. } => {
                Some((rule, effects, targets, cost, *sorcery))
            }
            _ => None,
        }
    }

    /// what a stack object of this ability resolves: rule text, effects, targets, whether it asks first
    pub fn on_stack(&self) -> Option<(&str, &[EffectDef], &[TargetDef], bool)> {
        if let Some((rule, effects, targets, optional)) = self.triggered() {
            return Some((rule, effects, targets, optional));
        }
        self.activated().map(|(rule, effects, targets, _, _)| (rule, effects, targets, false))
    }

    /// activated from hand, not the battlefield
    pub fn from_hand(&self) -> bool {
        matches!(
            self,
            AbilityDef::CyclingAbility { .. } | AbilityDef::ChannelAbility { .. } | AbilityDef::NinjutsuAbility { .. }
        )
    }

    /// activated from your graveyard, not the battlefield
    pub fn from_graveyard(&self) -> bool {
        matches!(self, AbilityDef::GraveyardActivatedAbility { .. })
    }

    pub fn is_enters_tapped(&self) -> bool {
        matches!(self, AbilityDef::EntersBattlefieldTappedAbility { .. })
    }

    /// a modal triggered ability's modes (mode 0 is its own effects and targets); empty if it isn't modal
    pub fn trigger_modes(&self) -> Vec<(&[EffectDef], &[TargetDef])> {
        match self {
            AbilityDef::TriggeredAbility { effects, targets, modes, .. } if !modes.is_empty() => {
                let mut all: Vec<(&[EffectDef], &[TargetDef])> = vec![(effects, targets)];
                all.extend(modes.iter().map(|m| (&m.effects[..], &m.targets[..])));
                all
            }
            _ => Vec::new(),
        }
    }

    /// the mana a `{T}: Add …` ability makes
    pub fn mana(&self) -> Option<Mana> {
        match self {
            AbilityDef::WhiteManaAbility { .. } => Some(Mana::of(Color::W, 1)),
            AbilityDef::BlueManaAbility { .. } => Some(Mana::of(Color::U, 1)),
            AbilityDef::BlackManaAbility { .. } => Some(Mana::of(Color::B, 1)),
            AbilityDef::RedManaAbility { .. } => Some(Mana::of(Color::R, 1)),
            AbilityDef::GreenManaAbility { .. } => Some(Mana::of(Color::G, 1)),
            AbilityDef::ColorlessManaAbility { amount, .. } => Some(Mana { colorless: *amount, ..Mana::default() }),
            AbilityDef::ColoredManaAbility { color, amount, .. } => Some(Mana::of(*color, *amount)),
            // at least one, as far as this can tell (the game counts the rest)
            AbilityDef::DynamicManaAbility { color: None, .. } => Some(Mana { colorless: 1, ..Mana::default() }),
            AbilityDef::DynamicManaAbility { color: Some(c), .. } => Some(Mana::of(*c, 1)),
            // (the game picks which as it lists the sources)
            AbilityDef::ConditionalColorManaAbility { otherwise, otherwise_amount, .. } => {
                Some(Mana::of(*otherwise, *otherwise_amount))
            }
            // one of each: whichever is needed
            AbilityDef::AnyColorManaAbility { amount, .. } => Some(Mana { colored: [*amount; 5], ..Mana::default() }),
            AbilityDef::ActivateIfConditionManaAbility { color, .. } => Some(Mana::of(*color, 1)),
            AbilityDef::ActivateIfConditionAnyColorManaAbility { .. } => {
                Some(Mana { colored: [1; 5], ..Mana::default() })
            }
            // counted by the game (devotion)
            AbilityDef::NykthosShrineToNyxManaAbility { .. } => Some(Mana { colored: [1; 5], ..Mana::default() }),
            // no commander: no mana
            AbilityDef::CommanderColorIdentityManaAbility { .. } => Some(Mana::default()),
            // counted by the game (the colors among them)
            AbilityDef::AnyColorPermanentTypesManaAbility { .. } => Some(Mana { colored: [1; 5], ..Mana::default() }),
            AbilityDef::ConditionalManaAbility { colorless: true, amount, .. } => {
                Some(Mana { colorless: *amount, ..Mana::default() })
            }
            AbilityDef::ConditionalManaAbility { color: Some(c), amount, .. } => Some(Mana::of(*c, *amount)),
            AbilityDef::ConditionalManaAbility { color: None, amount, .. } => {
                Some(Mana { colored: [*amount; 5], ..Mana::default() })
            }
            _ => None,
        }
    }

    /// what `ManaUtil.tryToAutoPay` sees in a mana ability
    pub fn mana_kind(&self) -> ManaKind {
        match self {
            // "two mana of any one color" is a `SimpleManaAbility` with an `AddManaOfAnyColorEffect` (Gift of Paradise)
            AbilityDef::AnyColorManaAbility { amount, .. } if *amount != 1 => ManaKind::Other,
            AbilityDef::AnyColorManaAbility { .. } => ManaKind::AnyColor,
            AbilityDef::ActivateIfConditionManaAbility { .. }
            | AbilityDef::ActivateIfConditionAnyColorManaAbility { .. }
            | AbilityDef::NykthosShrineToNyxManaAbility { .. }
            | AbilityDef::CommanderColorIdentityManaAbility { .. }
            | AbilityDef::AnyColorPermanentTypesManaAbility { .. }
            | AbilityDef::ConditionalManaAbility { .. }
            | AbilityDef::ConditionalColorManaAbility { .. }
            | AbilityDef::DynamicManaAbility { .. } => ManaKind::Other,
            // a `BasicManaAbility` makes one mana; "{T}: Add {G}{G}" is a `SimpleManaAbility`
            AbilityDef::ColoredManaAbility { amount, .. } if *amount != 1 => ManaKind::Other,
            // `SimpleManaAbility`s with a `ConditionalManaEffect` (Ugin's Labyrinth, Gemstone Caverns) or {C}{C}
            AbilityDef::ColorlessManaAbility { amount, two_if_exiled, any_color_with, .. }
                if *amount != 1 || *two_if_exiled || any_color_with.is_some() =>
            {
                ManaKind::Other
            }
            _ => ManaKind::Basic,
        }
    }

    /// whether activating this mana ability taps its source
    pub fn taps_source(&self) -> bool {
        !matches!(
            self,
            AbilityDef::ColoredManaAbility { no_tap: true, .. }
                | AbilityDef::ColorlessManaAbility { no_tap: true, .. }
                | AbilityDef::AnyColorManaAbility { no_tap: true, .. }
                | AbilityDef::DynamicManaAbility { no_tap: true, .. }
        )
    }

    /// the mana a mana ability's cost asks for (Castle Garenbrig's {2}{G}{G}; not Nykthos's, paid its own way)
    pub fn mana_ability_mana_cost(&self) -> Option<&str> {
        if matches!(self, AbilityDef::NykthosShrineToNyxManaAbility { .. }) {
            return None;
        }
        self.mana_costs().iter().find_map(|c| match c {
            CostDef::ManaCost { cost } => Some(cost.as_str()),
            _ => None,
        })
    }

    /// a mana ability's costs besides tapping it
    pub fn mana_costs(&self) -> &[CostDef] {
        match self {
            AbilityDef::AnyColorManaAbility { cost, .. }
            | AbilityDef::WhiteManaAbility { cost, .. }
            | AbilityDef::BlueManaAbility { cost, .. }
            | AbilityDef::BlackManaAbility { cost, .. }
            | AbilityDef::RedManaAbility { cost, .. }
            | AbilityDef::GreenManaAbility { cost, .. }
            | AbilityDef::ColorlessManaAbility { cost, .. }
            | AbilityDef::ColoredManaAbility { cost, .. }
            | AbilityDef::ConditionalManaAbility { cost, .. }
            | AbilityDef::DynamicManaAbility { cost, .. }
            | AbilityDef::ActivateIfConditionAnyColorManaAbility { cost, .. }
            | AbilityDef::NykthosShrineToNyxManaAbility { cost, .. } => cost,
            _ => &[],
        }
    }

    /// a mana ability's "activate only if"
    pub fn mana_condition(&self) -> Option<&Condition> {
        match self {
            AbilityDef::ActivateIfConditionManaAbility { condition, .. }
            | AbilityDef::ActivateIfConditionAnyColorManaAbility { condition, .. } => Some(condition),
            _ => None,
        }
    }

    /// what a mana ability does besides making mana
    pub fn mana_effects(&self) -> &[EffectDef] {
        match self {
            AbilityDef::WhiteManaAbility { effects, .. }
            | AbilityDef::BlueManaAbility { effects, .. }
            | AbilityDef::BlackManaAbility { effects, .. }
            | AbilityDef::RedManaAbility { effects, .. }
            | AbilityDef::GreenManaAbility { effects, .. }
            | AbilityDef::ColorlessManaAbility { effects, .. }
            | AbilityDef::AnyColorManaAbility { effects, .. } => effects,
            _ => &[],
        }
    }
}

/// A card as written by the transpiler.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct CardJson {
    pub name: String,
    #[serde(default)]
    pub cost: String,
    pub types: Vec<CardType>,
    #[serde(default)]
    pub subtypes: Vec<String>,
    #[serde(default)]
    pub supertypes: Vec<String>,
    /// colors set by the card (a land's are none); when absent, the cost's
    #[serde(default)]
    pub colors: Option<Vec<Color>>,
    #[serde(default)]
    pub power: Option<i32>,
    #[serde(default)]
    pub toughness: Option<i32>,
    #[serde(default)]
    pub abilities: Vec<AbilityDef>,
    #[serde(default)]
    pub spell: Option<SpellDef>,
    /// a planeswalker's starting loyalty
    #[serde(default)]
    pub loyalty: Option<i32>,
    /// a battle's starting defense
    #[serde(default)]
    pub defense: Option<i32>,
    /// an adventurer card's adventure: its own name, types, cost and spell
    #[serde(default)]
    pub adventure: Option<Box<CardJson>>,
    /// a modal double-faced card's back face
    #[serde(default)]
    pub back: Option<Box<CardJson>>,
    /// a transforming double-faced card's back face
    #[serde(default)]
    pub transform_back: Option<Box<CardJson>>,
    /// an omen card's spell (its `adventure`)
    #[serde(default)]
    pub omen: bool,
    /// an aftermath split card: its `room` halves are spells, the right one cast only from the graveyard
    #[serde(default)]
    pub aftermath: bool,
    /// a preparation card's prepare spell (`PrepareCard`)
    #[serde(default)]
    pub prepare: Option<Box<CardJson>>,
    /// a Room's halves, left then right (`RoomCard`)
    #[serde(default)]
    pub room: Vec<CardJson>,
    /// the set of the printing a deck gets by name
    #[serde(default)]
    pub set: String,
    #[serde(default)]
    pub hints: Vec<HintDef>,
    /// the index of the ability each hint is of, in its abilities
    #[serde(default)]
    pub hint_abilities: Vec<usize>,
    /// changeling: every creature type, in every zone (`ChangelingAbility`)
    #[serde(default)]
    pub all_creature_types: bool,
}

/// A hint an ability adds to a card's rules in a view (`mage.abilities.hint`), in the order of its abilities.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(tag = "h", rename_all = "snake_case")]
pub enum HintDef {
    /// `ConditionHint`: the true or false text, with a good or bad icon and a color
    Condition {
        condition: Condition,
        #[serde(default)]
        icons: bool,
        #[serde(default, rename = "trueText")]
        true_text: Option<String>,
        #[serde(default, rename = "falseText")]
        false_text: Option<String>,
        #[serde(default, rename = "trueColor")]
        true_color: Option<String>,
        #[serde(default, rename = "falseColor")]
        false_color: Option<String>,
    },
    /// `ValueHint`: "name: value"
    Value {
        #[serde(default)]
        name: String,
        value: Value,
    },
    /// a hint class of its own (`CanPlayAdditionalLandsHint`, `DayNightHint`…)
    Named { name: String },
    /// one the transpiler doesn't know: shows nothing
    Unknown {
        #[serde(default)]
        of: String,
        #[serde(default)]
        why: String,
    },
}

/// Subtypes that aren't creature types (`SubType`'s other `SubTypeSet`s), for "every creature type"
pub const NON_CREATURE_TYPES: &[&str] = &[
    "Adventure",
    "Arcane",
    "Lesson",
    "Omen",
    "Trap",
    "Vengeance",
    "Siege",
    "Forest",
    "Island",
    "Mountain",
    "Plains",
    "Swamp",
    "Cave",
    "Desert",
    "Gate",
    "Lair",
    "Locus",
    "Mine",
    "Planet",
    "Power-Plant",
    "Sphere",
    "Tower",
    "Town",
    "Urza's",
    "Aura",
    "Background",
    "Cartouche",
    "Case",
    "Class",
    "Curse",
    "Plan",
    "Role",
    "Room",
    "Rune",
    "Saga",
    "Shard",
    "Shrine",
    "Attraction",
    "Blood",
    "Bobblehead",
    "Book",
    "Clue",
    "Communicator",
    "Contraption",
    "Equipment",
    "Food",
    "Fortification",
    "Gold",
    "Heartwood",
    "Incubator",
    "Infinity",
    "Junk",
    "Lander",
    "Map",
    "Mutagen",
    "Powerstone",
    "Spacecraft",
    "Stone",
    "Treasure",
    "Vehicle",
    "Vibranium",
    "Ajani",
    "Aminatou",
    "Angrath",
    "Arlinn",
    "Arzakon",
    "Ashiok",
    "Aurra",
    "Bahamut",
    "Basri",
    "Bolas",
    "Calix",
    "Chandra",
    "Comet",
    "Dack",
    "Dakkon",
    "Daretti",
    "Davriel",
    "Dellian",
    "Dihada",
    "Domri",
    "Dooku",
    "Dovin",
    "Dyfed",
    "Ellywick",
    "Elminster",
    "Elspeth",
    "Estrid",
    "Feroz",
    "Freyalise",
    "Garruk",
    "Gideon",
    "Greensleeves",
    "Grist",
    "Guff",
    "Huatli",
    "Inzerva",
    "Jace",
    "Jared",
    "Jaya",
    "Jeska",
    "Kaito",
    "Karn",
    "Kasmina",
    "Kaya",
    "Kiora",
    "Koth",
    "Liliana",
    "Lolth",
    "Luke",
    "Lukka",
    "Minsc",
    "Mordenkainen",
    "Nahiri",
    "Narset",
    "Niko",
    "Nissa",
    "Nixilis",
    "Obi-Wan",
    "Oko",
    "Quintorius",
    "Ral",
    "Rey",
    "Rowan",
    "Saheeli",
    "Samut",
    "Sarkhan",
    "Serra",
    "Sidious",
    "Sifa",
    "Sivitri",
    "Snoke",
    "Sorin",
    "Szat",
    "Tamiyo",
    "Tasha",
    "Teferi",
    "Teyo",
    "Tezzeret",
    "Tibalt",
    "Tyvar",
    "Ugin",
    "Urza",
    "Venser",
    "Vivien",
    "Vraska",
    "Vronos",
    "Will",
    "Windgrace",
    "Worzel",
    "Wrenn",
    "Xenagos",
    "Yanggu",
    "Yanling",
    "Yoda",
    "Zariel",
];

/// A card's printed characteristics and abilities, shared by every copy of it and never changed.
#[derive(Clone, Debug, PartialEq)]
pub struct CardDef {
    pub name: String,
    /// the set of the printing a deck gets by name (the tokens it makes look for images there)
    pub set: String,
    pub cost: ManaCost,
    pub types: Vec<CardType>,
    pub subtypes: Vec<String>,
    pub supertypes: Vec<String>,
    pub colors: Vec<Color>,
    pub power: i32,
    pub toughness: i32,
    pub abilities: Vec<AbilityDef>,
    pub spell: SpellDef,
    pub loyalty: Option<i32>,
    pub defense: Option<i32>,
    /// every creature type (changeling, Mutavault)
    pub all_creature_types: bool,
    /// an adventurer card's adventure, cast instead of the card from hand (`AdventureCard`)
    pub adventure: Option<Arc<CardDef>>,
    /// a modal double-faced card's back face, played instead of the front (`ModalDoubleFacedCard`)
    pub back: Option<Arc<CardDef>>,
    /// a transforming double-faced card's back face, what the permanent transforms into
    /// (`TransformingDoubleFacedCard`)
    pub transform_back: Option<Arc<CardDef>>,
    /// an omen card's spell: once it resolves, the card is shuffled into its owner's library (`OmenCard`)
    pub omen: bool,
    /// an aftermath split card (`SplitCard`, `SPLIT_AFTERMATH`): its `room` halves are spells, the left cast from
    /// hand, the right only from the graveyard and then exiled
    pub aftermath: bool,
    /// a preparation card's prepare spell: what a prepared copy of it is (`PrepareCard`)
    pub prepare: Option<Arc<CardDef>>,
    /// a Room's halves, left then right: each cast as a spell, and unlocked as a door on the battlefield (`RoomCard`)
    pub room: Vec<Arc<CardDef>>,
    /// cast for an alternative cost (cleave, blitz, flashback…): the printed cost, which still gives the mana value
    pub printed_cost: Option<ManaCost>,
    /// the hints its abilities add to its rules in a view (`Ability.getHints`)
    pub hints: Vec<HintDef>,
    /// the index of the ability each hint is of (an ability's view on the stack shows its own)
    pub hint_abilities: Vec<usize>,
}

impl CardDef {
    /// Drops its power/toughness-defining abilities (layer 7a), as a copy "except it's N/N" does (707.9d,
    /// `MageObjectImpl.removePTCDA`)
    pub fn remove_pt_cda(&mut self) {
        let cda = |a: &AbilityDef| {
            matches!(a, AbilityDef::SimpleStaticAbility { effects, .. } if effects.iter().any(|e| matches!(e,
                EffectDef::SetBasePowerSourceEffect { .. }
                    | EffectDef::SetBasePowerToughnessPlusOneSourceEffect { .. }
                    | EffectDef::SetBasePowerToughnessAmountSourceEffect { .. }
                    | EffectDef::SetBasePowerToughnessSourceEffect { .. })))
        };
        let removed: Vec<usize> = (0..self.abilities.len()).filter(|i| cda(&self.abilities[*i])).collect();
        if removed.is_empty() {
            return;
        }
        // the hints go with their abilities, and the others' indices close up
        let mut hints = Vec::new();
        let mut of = Vec::new();
        for (i, h) in self.hints.iter().enumerate() {
            match self.hint_abilities.get(i).copied() {
                Some(k) if removed.contains(&k) => continue,
                Some(k) => of.push(k - removed.iter().filter(|r| **r < k).count()),
                None => {}
            }
            hints.push(h.clone());
        }
        self.hints = hints;
        self.hint_abilities = of;
        self.abilities.retain(|a| !cda(a));
    }

    /// The printed mana cost (not an alternative cost it's being cast for)
    pub fn printed(&self) -> &ManaCost {
        self.printed_cost.as_ref().unwrap_or(&self.cost)
    }

    /// The mana value, from the printed cost
    pub fn mana_value(&self) -> u32 {
        self.printed().mana_value()
    }

    pub fn from_json(mut json: CardJson) -> Result<CardDef, String> {
        // a card's other faces and its adventure are printed in its set (their tokens look for its images)
        for face in
            [&mut json.adventure, &mut json.back, &mut json.transform_back, &mut json.prepare].into_iter().flatten()
        {
            if face.set.is_empty() {
                face.set = json.set.clone();
            }
        }
        let cost = ManaCost::parse(&json.cost)?;
        let colors = json.colors.unwrap_or_else(|| cost.colors());
        Ok(CardDef {
            set: json.set,
            name: json.name,
            colors,
            cost,
            types: json.types,
            // an omen card's spell is an Omen (`OmenCard`'s spell subtype)
            subtypes: if json.omen {
                json.subtypes.into_iter().chain(["Omen".to_string()]).collect()
            } else {
                json.subtypes
            },
            supertypes: json.supertypes,
            power: json.power.unwrap_or(0),
            toughness: json.toughness.unwrap_or(0),
            loyalty: json.loyalty,
            defense: json.defense,
            all_creature_types: json.all_creature_types,
            abilities: json.abilities,
            spell: json.spell.unwrap_or_default(),
            adventure: match json.adventure {
                Some(a) => Some(Arc::new(CardDef::from_json(*a)?)),
                None => None,
            },
            back: match json.back {
                Some(a) => Some(Arc::new(CardDef::from_json(*a)?)),
                None => None,
            },
            transform_back: match json.transform_back {
                Some(a) => Some(Arc::new(CardDef::from_json(*a)?)),
                None => None,
            },
            omen: json.omen,
            aftermath: json.aftermath,
            prepare: match json.prepare {
                Some(a) => Some(Arc::new(CardDef::from_json(*a)?)),
                None => None,
            },
            room: json.room.into_iter().map(|h| CardDef::from_json(h).map(Arc::new)).collect::<Result<_, _>>()?,
            printed_cost: None,
            hints: json.hints,
            hint_abilities: json.hint_abilities,
        })
    }

    pub fn is(&self, t: CardType) -> bool {
        self.types.contains(&t)
    }

    pub fn has(&self, ability: &AbilityDef) -> bool {
        self.abilities.contains(ability)
    }

    pub fn has_subtype(&self, s: &str) -> bool {
        self.subtypes.iter().any(|x| x == s)
    }

    /// The card cast with its flashback cost, if it has flashback
    pub fn flashback(&self) -> Option<CardDef> {
        // an aftermath card's right half
        if self.aftermath {
            return self.room.get(1).map(|h| (**h).clone());
        }
        self.abilities.iter().find_map(|a| match a {
            AbilityDef::FlashbackAbility { cost, costs, .. } => {
                let mut spell = self.spell.clone();
                spell.costs.extend(costs.iter().cloned());
                Some(CardDef {
                    cost: ManaCost::parse(cost).ok()?,
                    printed_cost: Some(self.printed().clone()),
                    spell,
                    ..self.clone()
                })
            }
            AbilityDef::HarmonizeAbility { cost, .. } => {
                let mut spell = self.spell.clone();
                spell.costs.push(CostDef::HarmonizeCost);
                Some(CardDef {
                    cost: ManaCost::parse(cost).ok()?,
                    printed_cost: Some(self.printed().clone()),
                    spell,
                    ..self.clone()
                })
            }
            _ => None,
        })
    }

    /// Plot: its ability index and plot cost
    pub fn plot(&self) -> Option<(usize, ManaCost)> {
        self.abilities.iter().enumerate().find_map(|(k, a)| match a {
            AbilityDef::PlotAbility { cost, .. } => Some((k, ManaCost::parse(cost).ok()?)),
            _ => None,
        })
    }

    /// Suspend: its ability index, time counters and cost
    pub fn suspend(&self) -> Option<(usize, i32, ManaCost)> {
        self.abilities.iter().enumerate().find_map(|(k, a)| match a {
            AbilityDef::SuspendAbility { time, cost, .. } => Some((k, *time, ManaCost::parse(cost).ok()?)),
            _ => None,
        })
    }

    /// Foretell: its ability index, and the card with its foretell cost
    pub fn foretold(&self) -> Option<(usize, CardDef)> {
        self.abilities.iter().enumerate().find_map(|(k, a)| match a {
            AbilityDef::ForetellAbility { cost, .. } => Some((
                k,
                CardDef {
                    cost: ManaCost::parse(cost).ok()?,
                    printed_cost: Some(self.printed().clone()),
                    ..self.clone()
                },
            )),
            _ => None,
        })
    }

    /// The card cast with its escape cost, if it has escape
    pub fn escape(&self) -> Option<CardDef> {
        self.abilities.iter().find_map(|a| match a {
            AbilityDef::EscapeAbility { cost, exile, .. } => {
                let mut spell = self.spell.clone();
                spell.costs.push(CostDef::ExileFromGraveCost { amount: *exile, filter: Filter::default() });
                Some(CardDef {
                    cost: ManaCost::parse(cost).ok()?,
                    printed_cost: Some(self.printed().clone()),
                    spell,
                    ..self.clone()
                })
            }
            _ => None,
        })
    }

    /// The spectacle ability's index and the card with its spectacle cost.
    pub fn spectacle(&self) -> Option<(usize, CardDef)> {
        self.abilities.iter().enumerate().find_map(|(k, a)| match a {
            AbilityDef::SpectacleAbility { cost, .. } => Some((
                k,
                CardDef {
                    cost: ManaCost::parse(cost).ok()?,
                    printed_cost: Some(self.printed().clone()),
                    ..self.clone()
                },
            )),
            _ => None,
        })
    }

    /// The overload ability's index and the card cast with it: its overload cost, no targets, the "each" effects.
    pub fn overload(&self) -> Option<(usize, CardDef)> {
        self.abilities.iter().enumerate().find_map(|(k, a)| match a {
            AbilityDef::OverloadAbility { cost, effects, .. } => {
                let mut spell = self.spell.clone();
                spell.effects = effects.clone();
                spell.targets.clear();
                Some((
                    k,
                    CardDef {
                        cost: ManaCost::parse(cost).ok()?,
                        printed_cost: Some(self.printed().clone()),
                        spell,
                        ..self.clone()
                    },
                ))
            }
            _ => None,
        })
    }

    /// The blitz ability's index and the card with its blitz costs.
    pub fn blitz(&self) -> Option<(usize, CardDef)> {
        self.abilities.iter().enumerate().find_map(|(k, a)| match a {
            AbilityDef::BlitzAbility { mana, cost, .. } => {
                let mut spell = self.spell.clone();
                spell.costs.extend(cost.iter().cloned());
                Some((
                    k,
                    CardDef {
                        cost: ManaCost::parse(mana).ok()?,
                        printed_cost: Some(self.printed().clone()),
                        spell,
                        ..self.clone()
                    },
                ))
            }
            _ => None,
        })
    }

    /// The cleave ability's index and the card with its cleave cost and effects.
    pub fn cleave(&self) -> Option<(usize, CardDef)> {
        self.abilities.iter().enumerate().find_map(|(k, a)| match a {
            AbilityDef::CleaveAbility { cost, effects, targets, .. } => {
                let mut spell = self.spell.clone();
                spell.effects = effects.clone();
                if let Some(targets) = targets {
                    spell.targets = targets.clone();
                }
                Some((
                    k,
                    CardDef {
                        cost: ManaCost::parse(cost).ok()?,
                        printed_cost: Some(self.printed().clone()),
                        spell,
                        ..self.clone()
                    },
                ))
            }
            _ => None,
        })
    }

    /// The warp ability's index and the card with its warp cost.
    pub fn warp(&self) -> Option<(usize, CardDef)> {
        self.abilities.iter().enumerate().find_map(|(k, a)| match a {
            AbilityDef::WarpAbility { cost, .. } => Some((
                k,
                CardDef {
                    cost: ManaCost::parse(cost).ok()?,
                    printed_cost: Some(self.printed().clone()),
                    ..self.clone()
                },
            )),
            _ => None,
        })
    }

    pub fn is_permanent_card(&self) -> bool {
        !self.is(CardType::Instant) && !self.is(CardType::Sorcery)
    }
}

/// The cards the engine has, by name.
#[derive(Clone, Debug, Default)]
pub struct CardDb {
    cards: HashMap<String, Arc<CardDef>>,
}

impl CardDb {
    /// Loads every card in a JSON array; a card that doesn't parse is left out and its error returned.
    pub fn load_json(text: &str) -> Result<(CardDb, Vec<String>), String> {
        let values: Vec<serde_json::Value> = serde_json::from_str(text).map_err(|e| e.to_string())?;
        let mut db = CardDb::default();
        let mut errors = Vec::new();
        for value in values {
            let name = value.get("name").and_then(|n| n.as_str()).unwrap_or("?").to_string();
            match serde_json::from_value::<CardJson>(value).map_err(|e| e.to_string()).and_then(CardDef::from_json) {
                Ok(def) => {
                    db.cards.insert(def.name.clone(), Arc::new(def));
                }
                Err(e) => errors.push(format!("{}: {}", name, e)),
            }
        }
        Ok((db, errors))
    }

    pub fn iter(&self) -> impl Iterator<Item = &Arc<CardDef>> {
        self.cards.values()
    }

    pub fn get(&self, name: &str) -> Option<Arc<CardDef>> {
        self.cards.get(name).cloned()
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mana_costs() {
        let c = ManaCost::parse("{2}{R}{R}").unwrap();
        assert_eq!(c.generic, 2);
        assert_eq!(c.colored[Color::R.index()], 2);
        assert_eq!(c.mana_value(), 4);
        assert_eq!(c.colors(), vec![Color::R]);
        assert!(ManaCost::parse("").unwrap().is_empty());
        let h = ManaCost::parse("{1}{W/U}").unwrap();
        assert_eq!(h.mana_value(), 2);
        assert!(ManaCost::parse("{W/X}").is_err());
    }

    #[test]
    fn unknown_classes_leave_a_card_out() {
        let (db, errors) = CardDb::load_json(
            r#"[{"name":"Shock","cost":"{R}","types":["INSTANT"],"spell":{"effects":[{"class":"DamageTargetEffect","amount":2}],"targets":[{"class":"TargetAnyTarget"}]}},
                {"name":"Odd","cost":"{1}","types":["ARTIFACT"],"abilities":[{"class":"SomethingNewAbility"}]}]"#,
        )
        .unwrap();
        assert!(db.get("Shock").is_some());
        assert!(db.get("Odd").is_none());
        assert_eq!(errors.len(), 1);
    }
}
