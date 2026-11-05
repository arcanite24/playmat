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
                n => {
                    cost.generic += n.parse::<u32>().map_err(|_| format!("mana symbol {{{}}} not supported", n))?;
                }
            }
            rest = &rest[end + 1..];
        }
        Ok(cost)
    }

    pub fn mana_value(&self) -> u32 {
        self.generic + self.colored.iter().sum::<u32>() + self.colorless
    }

    pub fn colors(&self) -> Vec<Color> {
        Color::ALL.iter().copied().filter(|c| self.colored[c.index()] > 0).collect()
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
    TargetAnyTarget,
    TargetPlayer,
    TargetOpponent,
    /// a spell on the stack, of the filter
    TargetSpell {
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
    },
    TargetPlayerOrPlaneswalker,
    /// a card in a graveyard (`TargetCardInGraveyard`, or `…InYourGraveyard` with the "you" owner predicate)
    TargetCardInGraveyard {
        #[serde(default)]
        filter: Filter,
        #[serde(default = "one")]
        min: usize,
        #[serde(default = "one")]
        max: usize,
    },
}

fn one() -> usize {
    1
}

/// How long a continuous effect lasts (Java's `Duration`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum Duration {
    EndOfTurn,
    WhileOnBattlefield,
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
    /// `TappedPredicate.TAPPED` (true) or `.UNTAPPED`
    Tapped {
        v: bool,
    },
    /// `AttackingPredicate`
    Attacking,
    /// attacking or blocking
    InCombat,
    /// `TokenPredicate.TRUE` (true) or `.FALSE`
    Token {
        v: bool,
    },
    /// `CounterType.X.getPredicate()`: has at least one counter of the kind
    Counter {
        name: String,
    },
    /// `AbilityPredicate(X.class)`: has the keyword
    Ability {
        a: Box<AbilityDef>,
    },
    /// `NamePredicate`
    Name {
        n: String,
    },
    /// `PermanentPredicate`: a permanent card (not an instant or sorcery)
    PermanentCard,
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
    /// this attacks (`AttacksTriggeredAbility`)
    Attacks,
    /// this dies (`DiesSourceTriggeredAbility`)
    Dies,
    EntersOrDies,
    EntersOrAttacks,
    /// a permanent of the filter enters (`EntersBattlefieldAllTriggeredAbility`, `…ControlledTriggeredAbility`,
    /// landfall)
    EntersAll {
        filter: Filter,
    },
    /// a permanent of the filter dies; with `include_self`, this too, whatever the filter
    DiesAll {
        filter: Filter,
        #[serde(default)]
        include_self: bool,
    },
    /// at the beginning of `step` ("upkeep", "combat", "end") of `who`'s turn: "you", "opponent" or "any"
    StepBegins {
        step: String,
        who: String,
    },
    /// you gain life (`GainLifeControllerTriggeredAbility`)
    YouGainLife,
    /// `who` ("you", "opponent", "any") casts a spell of the filter (`SpellCastControllerTriggeredAbility`, prowess)
    SpellCast {
        filter: Filter,
        who: String,
    },
    /// this deals combat damage to a player
    CombatDamageToPlayer,
    /// one or more opponents are dealt noncombat damage (Master of Barbs)
    OpponentsDealtNoncombatDamage,
}

/// Conditions (Java's `Condition`s), checked for the controller of a source.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(tag = "c", rename_all = "snake_case", deny_unknown_fields)]
pub enum Condition {
    /// `KickedCondition`: the spell was kicked (only a spell or what it put on the battlefield knows)
    Kicked,
    /// `RaidCondition`: you attacked this turn
    Raid,
    /// `MyTurnCondition`
    MyTurn,
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
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct TokenDef {
    pub name: String,
    pub types: Vec<CardType>,
    #[serde(default)]
    pub colors: Vec<Color>,
    #[serde(default)]
    pub subtypes: Vec<String>,
    #[serde(default)]
    pub power: i32,
    #[serde(default)]
    pub toughness: i32,
    #[serde(default)]
    pub abilities: Vec<AbilityDef>,
}

impl TokenDef {
    pub fn card(&self) -> CardDef {
        CardDef {
            name: self.name.clone(),
            cost: ManaCost::default(),
            types: self.types.clone(),
            subtypes: self.subtypes.clone(),
            supertypes: Vec::new(),
            colors: self.colors.clone(),
            power: self.power,
            toughness: self.toughness,
            abilities: self.abilities.clone(),
            spell: SpellDef::default(),
        }
    }
}

/// Costs of an activated ability, named after the Java cost classes.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(tag = "class")]
pub enum CostDef {
    ManaCost { cost: String },
    TapSourceCost,
    SacrificeSourceCost,
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

/// One-shot effects, named after the Java effect classes.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(tag = "class", deny_unknown_fields)]
pub enum EffectDef {
    /// deals `amount` damage to the ability's target
    DamageTargetEffect {
        amount: i32,
    },
    /// the controller draws `amount` cards
    DrawCardSourceControllerEffect {
        amount: i32,
    },
    /// the controller gains `amount` life
    GainLifeEffect {
        amount: i32,
    },
    /// destroys the target permanent
    DestroyTargetEffect,
    /// counters the target spell
    CounterTargetEffect,
    /// the target creature gets +power/+toughness
    BoostTargetEffect {
        power: i32,
        toughness: i32,
        duration: Duration,
    },
    /// the source gets +power/+toughness
    BoostSourceEffect {
        power: i32,
        toughness: i32,
        duration: Duration,
    },
    /// a static ability's effect: the permanents of `filter` its controller controls get +power/+toughness
    BoostControlledEffect {
        power: i32,
        toughness: i32,
        duration: Duration,
        filter: Filter,
        exclude_source: bool,
    },
    /// puts counters on the source
    AddCountersSourceEffect {
        counter: CounterDef,
    },
    /// puts counters on the target permanent
    AddCountersTargetEffect {
        counter: CounterDef,
    },
    /// the controller creates `amount` tokens; Kavaron Harrier's enter tapped and attacking and are sacrificed at end
    /// of combat
    CreateTokenEffect {
        token: TokenDef,
        amount: i32,
        #[serde(default)]
        tapped_and_attacking: bool,
        #[serde(default)]
        sacrifice_at_end_of_combat: bool,
    },
    /// deals damage equal to the source's power (as it last was on the battlefield) to the target
    DamageTargetSourcePowerEffect,
    /// `effect` if the condition holds, otherwise `otherwise`
    ConditionalOneShotEffect {
        #[serde(default)]
        effect: Option<Box<EffectDef>>,
        #[serde(default)]
        otherwise: Option<Box<EffectDef>>,
        condition: Condition,
    },
    /// deals `amount` damage to each player of `controller` ("opponent")
    DamagePlayersEffect {
        amount: i32,
        controller: String,
    },
    /// sacrifices the source
    SacrificeSourceEffect,
    /// the controller may pay `cost`; if they do, `effect`
    DoIfCostPaid {
        effect: Box<EffectDef>,
        cost: Vec<CostDef>,
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
    /// a static effect that applies only while the condition holds ("my_turn")
    ConditionalContinuousEffect {
        effect: Box<EffectDef>,
        condition: Condition,
    },
    /// the source's base power and toughness are each a count (layer 7a; "creatures_you_control")
    SetBasePowerToughnessSourceEffect {
        count: String,
    },
    /// if the target would die this turn, it's exiled instead
    ExileTargetIfDiesEffect,
    /// adds a subtype to the source (subtypes aren't in the fingerprint, and no rule reads them yet)
    AddCardSubTypeSourceEffect,
    /// as it enters, its controller may pay `life`; if they don't, it enters tapped
    TapSourceUnlessPaysEffect {
        life: i32,
    },
    /// the first target deals damage equal to its power to the second (Bite Down)
    DamageWithPowerFromOneToAnotherTargetEffect,
    /// each opponent loses `amount` life
    LoseLifeOpponentsEffect {
        amount: i32,
    },
    /// the controller loses `amount` life
    LoseLifeSourceControllerEffect {
        amount: i32,
    },
    /// the target players lose `amount` life
    LoseLifeTargetEffect {
        amount: i32,
    },
    /// destroys every permanent of `filter`
    DestroyAllEffect {
        filter: Filter,
    },
    /// deals `amount` damage to every permanent of `filter`
    DamageAllEffect {
        amount: i32,
        filter: Filter,
    },
    /// the permanents of `filter` get +power/+toughness (static: while on the battlefield; resolving: those there now)
    BoostAllEffect {
        power: i32,
        toughness: i32,
        duration: Duration,
        filter: Filter,
        exclude_source: bool,
    },
    /// puts counters on every permanent of `filter`
    AddCountersAllEffect {
        counter: CounterDef,
        filter: Filter,
    },
    /// exiles the targets
    ExileTargetEffect,
    /// returns the targets to their owners' hands
    ReturnToHandTargetEffect,
    TapTargetEffect,
    UntapTargetEffect,
    /// taps the source
    TapSourceEffect,
    /// counters the target spell unless its controller pays `cost`
    CounterUnlessPaysEffect {
        cost: Vec<CostDef>,
    },
    /// the source deals `amount` damage to its controller
    DamageControllerEffect {
        amount: i32,
    },
    /// the target players draw `amount` cards
    DrawCardTargetEffect {
        amount: i32,
    },
    /// the controller scries `amount`
    ScryEffect {
        amount: i32,
    },
    /// the controller surveils `amount`
    SurveilEffect {
        amount: i32,
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
    },
    /// a static ability's effect: the permanents of `filter` have `ability`
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
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct ModeDef {
    #[serde(default)]
    pub effects: Vec<EffectDef>,
    #[serde(default)]
    pub targets: Vec<TargetDef>,
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
        /// the effects act on the object or player the event was about (`SetTargetPointer`)
        #[serde(default)]
        set_target_pointer: bool,
        rule: String,
    },
    EntersBattlefieldTappedAbility {
        rule: String,
    },
    /// a replacement effect applied as it enters (shock lands)
    AsEntersBattlefieldAbility {
        effects: Vec<EffectDef>,
        rule: String,
    },
    /// an optional additional cost asked for as the spell is cast
    KickerAbility {
        cost: String,
        rule: String,
    },
    /// enters with its effects applied (a replacement effect), if the condition holds
    EntersBattlefieldAbility {
        effects: Vec<EffectDef>,
        #[serde(default)]
        condition: Option<Condition>,
        rule: String,
    },
    ActivateAsSorceryActivatedAbility {
        effects: Vec<EffectDef>,
        cost: Vec<CostDef>,
        #[serde(default)]
        targets: Vec<TargetDef>,
        rule: String,
    },
    SimpleActivatedAbility {
        effects: Vec<EffectDef>,
        cost: Vec<CostDef>,
        #[serde(default)]
        targets: Vec<TargetDef>,
        rule: String,
    },
    FlyingAbility,
    ReachAbility,
    HasteAbility,
    VigilanceAbility,
    TrampleAbility,
    FirstStrikeAbility,
    DoubleStrikeAbility,
    LifelinkAbility,
    DeathtouchAbility,
    DefenderAbility,
    MenaceAbility,
    FlashAbility,
    HexproofAbility,
    IndestructibleAbility,
    /// {T}: Add {W}. The basic land types' mana abilities are these too.
    WhiteManaAbility {
        /// what else activating it does (a pain land's damage)
        #[serde(default)]
        effects: Vec<EffectDef>,
    },
    BlueManaAbility {
        #[serde(default)]
        effects: Vec<EffectDef>,
    },
    BlackManaAbility {
        #[serde(default)]
        effects: Vec<EffectDef>,
    },
    RedManaAbility {
        #[serde(default)]
        effects: Vec<EffectDef>,
    },
    GreenManaAbility {
        #[serde(default)]
        effects: Vec<EffectDef>,
    },
    ColorlessManaAbility {
        #[serde(default)]
        effects: Vec<EffectDef>,
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
    pub fn intervening_if(&self) -> Option<&Condition> {
        match self {
            AbilityDef::TriggeredAbility { condition, .. } => condition.as_ref(),
            _ => None,
        }
    }

    /// an activated ability's rule text, effects, targets, costs and whether it's sorcery speed
    pub fn activated(&self) -> Option<Activated<'_>> {
        match self {
            AbilityDef::ActivateAsSorceryActivatedAbility { effects, cost, targets, rule } => {
                Some((rule, effects, targets, cost, true))
            }
            AbilityDef::SimpleActivatedAbility { effects, cost, targets, rule } => {
                Some((rule, effects, targets, cost, false))
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

    pub fn is_enters_tapped(&self) -> bool {
        matches!(self, AbilityDef::EntersBattlefieldTappedAbility { .. })
    }

    /// the mana a `{T}: Add …` ability makes
    pub fn mana(&self) -> Option<Mana> {
        match self {
            AbilityDef::WhiteManaAbility { .. } => Some(Mana::of(Color::W, 1)),
            AbilityDef::BlueManaAbility { .. } => Some(Mana::of(Color::U, 1)),
            AbilityDef::BlackManaAbility { .. } => Some(Mana::of(Color::B, 1)),
            AbilityDef::RedManaAbility { .. } => Some(Mana::of(Color::R, 1)),
            AbilityDef::GreenManaAbility { .. } => Some(Mana::of(Color::G, 1)),
            AbilityDef::ColorlessManaAbility { .. } => Some(Mana { colorless: 1, ..Mana::default() }),
            _ => None,
        }
    }

    /// what a mana ability does besides making mana
    pub fn mana_effects(&self) -> &[EffectDef] {
        match self {
            AbilityDef::WhiteManaAbility { effects }
            | AbilityDef::BlueManaAbility { effects }
            | AbilityDef::BlackManaAbility { effects }
            | AbilityDef::RedManaAbility { effects }
            | AbilityDef::GreenManaAbility { effects }
            | AbilityDef::ColorlessManaAbility { effects } => effects,
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
}

/// A card's printed characteristics and abilities, shared by every copy of it and never changed.
#[derive(Clone, Debug, PartialEq)]
pub struct CardDef {
    pub name: String,
    pub cost: ManaCost,
    pub types: Vec<CardType>,
    pub subtypes: Vec<String>,
    pub supertypes: Vec<String>,
    pub colors: Vec<Color>,
    pub power: i32,
    pub toughness: i32,
    pub abilities: Vec<AbilityDef>,
    pub spell: SpellDef,
}

impl CardDef {
    pub fn from_json(json: CardJson) -> Result<CardDef, String> {
        let cost = ManaCost::parse(&json.cost)?;
        let colors = json.colors.unwrap_or_else(|| cost.colors());
        Ok(CardDef {
            name: json.name,
            colors,
            cost,
            types: json.types,
            subtypes: json.subtypes,
            supertypes: json.supertypes,
            power: json.power.unwrap_or(0),
            toughness: json.toughness.unwrap_or(0),
            abilities: json.abilities,
            spell: json.spell.unwrap_or_default(),
        })
    }

    pub fn is(&self, t: CardType) -> bool {
        self.types.contains(&t)
    }

    pub fn has(&self, ability: &AbilityDef) -> bool {
        self.abilities.contains(ability)
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
        assert!(ManaCost::parse("{W/U}").is_err());
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
