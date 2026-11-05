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
}

/// The spell ability of an instant or sorcery: what it does when it resolves.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct SpellDef {
    #[serde(default)]
    pub effects: Vec<EffectDef>,
    #[serde(default)]
    pub targets: Vec<TargetDef>,
}

/// Abilities of a card, named after the Java ability classes.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(tag = "class")]
pub enum AbilityDef {
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

impl AbilityDef {
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
