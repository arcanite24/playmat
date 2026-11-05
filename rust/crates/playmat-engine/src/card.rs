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

/// Targets, named after the Java target classes.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(tag = "class")]
pub enum TargetDef {
    /// any target: a creature, player, planeswalker or battle
    TargetAnyTarget,
    TargetCreaturePermanent,
    TargetPlayer,
    TargetOpponent,
    /// a spell on the stack
    TargetSpell,
    /// a creature you control
    TargetControlledCreaturePermanent,
    /// a permanent of the card's own filter
    TargetPermanent {
        filter: Filter,
    },
    TargetCreatureOrPlaneswalker,
}

/// How long a continuous effect lasts (Java's `Duration`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum Duration {
    EndOfTurn,
    WhileOnBattlefield,
}

/// Which permanents a filter takes (Java's `StaticFilters`, or a card's own filter field). "You" is the filter's
/// source's controller (or the player choosing targets).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Filter {
    Creatures,
    /// `FILTER_EACH_CONTROLLED_CREATURE_P1P1`: each creature you control with a +1/+1 counter on it
    ControlledCreaturesWithP1p1,
    /// `FILTER_ANOTHER_CREATURE_YOU_CONTROL`
    AnotherCreatureYouControl,
    /// a creature or planeswalker you don't control
    OpponentsCreatureOrPlaneswalker,
    CreaturesOrPlaneswalkers,
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

/// One-shot effects, named after the Java effect classes.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(tag = "class")]
pub enum EffectDef {
    /// deals `amount` damage to the ability's target
    DamageTargetEffect { amount: i32 },
    /// the controller draws `amount` cards
    DrawCardSourceControllerEffect { amount: i32 },
    /// the controller gains `amount` life
    GainLifeEffect { amount: i32 },
    /// destroys the target permanent
    DestroyTargetEffect,
    /// counters the target spell
    CounterTargetEffect,
    /// the target creature gets +power/+toughness
    BoostTargetEffect { power: i32, toughness: i32, duration: Duration },
    /// the source gets +power/+toughness
    BoostSourceEffect { power: i32, toughness: i32, duration: Duration },
    /// a static ability's effect: the permanents of `filter` its controller controls get +power/+toughness
    BoostControlledEffect { power: i32, toughness: i32, duration: Duration, filter: Filter, exclude_source: bool },
    /// puts counters on the source
    AddCountersSourceEffect { counter: CounterDef },
    /// puts counters on the target permanent
    AddCountersTargetEffect { counter: CounterDef },
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
    ConditionalOneShotEffect { effect: Box<EffectDef>, otherwise: Box<EffectDef>, condition: String },
    /// deals `amount` damage to each player of `controller` ("opponent")
    DamagePlayersEffect { amount: i32, controller: String },
    /// sacrifices the source
    SacrificeSourceEffect,
    /// the controller may pay `cost`; if they do, `effect`
    DoIfCostPaid { effect: Box<EffectDef>, cost: Vec<CostDef> },
    /// the first target deals damage equal to its power to the second (Bite Down)
    DamageWithPowerFromOneToAnotherTargetEffect,
    /// a static ability's effect: the permanents of `filter` have `ability`
    GainAbilityAllEffect { ability: Box<AbilityDef>, duration: Duration, filter: Filter },
}

/// The spell ability of an instant or sorcery: what it does when it resolves.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct SpellDef {
    #[serde(default)]
    pub effects: Vec<EffectDef>,
    #[serde(default)]
    pub targets: Vec<TargetDef>,
}

/// Abilities of a card, named after the Java ability classes, in the order Java lists them (decision logs name an
/// ability by its place in that list). `rule` is the rule text as Java prints it, `{this}` and all.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(tag = "class")]
pub enum AbilityDef {
    /// casting the card (every nonland card has one)
    SpellAbility,
    /// playing the card (every land has one)
    PlayLandAbility,
    EntersBattlefieldTriggeredAbility {
        effects: Vec<EffectDef>,
        #[serde(default)]
        optional: bool,
        rule: String,
    },
    AttacksTriggeredAbility {
        effects: Vec<EffectDef>,
        #[serde(default)]
        optional: bool,
        rule: String,
    },
    SimpleStaticAbility {
        effects: Vec<EffectDef>,
        rule: String,
    },
    EntersBattlefieldTappedAbility {
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
        condition: Option<String>,
        rule: String,
    },
    /// whenever a permanent of `filter` enters
    EntersBattlefieldAllTriggeredAbility {
        effects: Vec<EffectDef>,
        filter: Filter,
        #[serde(default)]
        optional: bool,
        rule: String,
    },
    /// whenever a land you control enters
    LandfallAbility {
        effects: Vec<EffectDef>,
        #[serde(default)]
        optional: bool,
        rule: String,
    },
    /// when this dies
    DiesSourceTriggeredAbility {
        effects: Vec<EffectDef>,
        #[serde(default)]
        optional: bool,
        rule: String,
    },
    /// whenever you cast a noncreature spell, this gets +1/+1 until end of turn
    ProwessAbility {
        rule: String,
    },
    /// at the beginning of the end step of `controller` ("you", or "any" player's), if the condition holds
    BeginningOfEndStepTriggeredAbility {
        controller: String,
        effects: Vec<EffectDef>,
        #[serde(default)]
        optional: bool,
        #[serde(default)]
        condition: Option<String>,
        rule: String,
    },
    /// whenever one or more opponents are dealt noncombat damage (Master of Barbs)
    DamagedOpponentsNoncombatTriggeredAbility {
        effects: Vec<EffectDef>,
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
    WhiteManaAbility,
    BlueManaAbility,
    BlackManaAbility,
    RedManaAbility,
    GreenManaAbility,
    ColorlessManaAbility,
}

const PROWESS: &[EffectDef] = &[EffectDef::BoostSourceEffect { power: 1, toughness: 1, duration: Duration::EndOfTurn }];

/// An activated ability's parts: rule text, effects, targets, costs, sorcery speed.
pub type Activated<'a> = (&'a str, &'a [EffectDef], &'a [TargetDef], &'a [CostDef], bool);

impl AbilityDef {
    /// a triggered ability's rule text, effects and whether it's a "may" ability
    pub fn triggered(&self) -> Option<(&str, &[EffectDef], bool)> {
        match self {
            AbilityDef::EntersBattlefieldTriggeredAbility { effects, optional, rule }
            | AbilityDef::AttacksTriggeredAbility { effects, optional, rule }
            | AbilityDef::EntersBattlefieldAllTriggeredAbility { effects, optional, rule, .. }
            | AbilityDef::LandfallAbility { effects, optional, rule }
            | AbilityDef::DiesSourceTriggeredAbility { effects, optional, rule } => Some((rule, effects, *optional)),
            AbilityDef::BeginningOfEndStepTriggeredAbility { effects, optional, rule, .. } => {
                Some((rule, effects, *optional))
            }
            AbilityDef::DamagedOpponentsNoncombatTriggeredAbility { effects, rule } => Some((rule, effects, false)),
            AbilityDef::ProwessAbility { rule } => Some((rule, PROWESS, false)),
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
        if let Some((rule, effects, optional)) = self.triggered() {
            return Some((rule, effects, &[], optional));
        }
        self.activated().map(|(rule, effects, targets, _, _)| (rule, effects, targets, false))
    }

    pub fn is_enters_tapped(&self) -> bool {
        matches!(self, AbilityDef::EntersBattlefieldTappedAbility { .. })
    }

    /// the mana a `{T}: Add …` ability makes
    pub fn mana(&self) -> Option<Mana> {
        match self {
            AbilityDef::WhiteManaAbility => Some(Mana::of(Color::W, 1)),
            AbilityDef::BlueManaAbility => Some(Mana::of(Color::U, 1)),
            AbilityDef::BlackManaAbility => Some(Mana::of(Color::B, 1)),
            AbilityDef::RedManaAbility => Some(Mana::of(Color::R, 1)),
            AbilityDef::GreenManaAbility => Some(Mana::of(Color::G, 1)),
            AbilityDef::ColorlessManaAbility => Some(Mana { colorless: 1, ..Mana::default() }),
            _ => None,
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
