//! Per-card handlers (docs/rust/Engine.md, "Per-card handlers"): Rust written by hand for a class a card file
//! declares for itself (`EtaliPrimalStormEffect`), one file per card under `src/cards/`.
//!
//! A handler file is a child module of `rules`, so it can call the rules' own helpers (`deal_damage`,
//! `choose_cards`, `put_onto_battlefield`, `apply_each_effect`…) with `use super::*;`. It declares
//!
//! ```ignore
//! pub const HANDLERS: &[Handler] = &[Handler::effect("EtaliPrimalStorm.EtaliPrimalStormEffect", apply)];
//! ```
//!
//! naming the Java class it stands for as `OuterCardClass.InnerClass` (or the class alone for one in the framework).
//! `build.rs` lists every file here, so adding a card never touches a shared file, and `rust/tools/transpile.py`
//! reads the same names: a card whose class has a handler gets `{"class": "CardEffect", "handler": …}` (or a
//! condition's `{"c": "card", …}`, a value's `{"v": "card", …}`) in cards.json. A name cards.json uses that no file
//! registers leaves the card out of the engine, as any JSON this build doesn't know does.

// a handler's `use super::*;` brings in the rules, their helpers and imports with this module's own items
#[allow(unused_imports)]
use super::*;
use crate::card::{EffectDef, Filter, TokenDef};
use crate::decide::Decider;
use crate::state::{Game, ObjId, Seat, StackItem, Target};
use std::collections::HashMap;
use std::sync::OnceLock;

/// What a one-shot effect handler gets: the resolving spell or ability, its targets that are still legal (one group
/// per target of the ability, in order), the source with its zone-change count as the ability was put on the stack,
/// the Java constructor's arguments as their source text (most classes have none), and what the transpiler made of
/// the Java the handler asked for (`Handler::tokens`, `effects`, `filters`), in the order it was asked.
#[allow(dead_code)]
pub(crate) struct EffectCx<'a> {
    pub item: &'a StackItem,
    pub legal: &'a [Vec<Target>],
    pub source: (ObjId, u32),
    pub args: &'a [String],
    pub tokens: &'a [TokenDef],
    pub effects: &'a [EffectDef],
    pub filters: &'a [Filter],
}

#[allow(dead_code)] // a kit for the handlers: not every handler uses every part
impl EffectCx<'_> {
    /// the ability's controller (Java's `source.getControllerId()`)
    pub fn controller(&self) -> Seat {
        self.item.controller
    }

    /// the source object (Java's `source.getSourceId()`)
    pub fn source_id(&self) -> ObjId {
        self.source.0
    }

    /// the objects of target group `i` that are still legal (Java's target pointer, for the first group)
    pub fn objs(&self, i: usize) -> Vec<ObjId> {
        self.legal
            .get(i)
            .map(|ts| ts.iter().filter_map(|t| if let Target::Obj(o) = t { Some(*o) } else { None }).collect())
            .unwrap_or_default()
    }

    /// the first object of target group `i`, if it's still legal
    pub fn obj(&self, i: usize) -> Option<ObjId> {
        self.objs(i).first().copied()
    }

    /// the first player of target group `i`
    pub fn player(&self, i: usize) -> Option<Seat> {
        self.legal.get(i)?.iter().find_map(|t| if let Target::Player(p) = t { Some(*p) } else { None })
    }

    /// the first target of group `i`, player or object
    pub fn target(&self, i: usize) -> Option<Target> {
        self.legal.get(i)?.first().copied()
    }

    /// The source is still the permanent that put this ability on the stack
    pub fn source_on_battlefield(&self, game: &Game) -> bool {
        let o = game.obj(self.source.0);
        o.zone == crate::state::Zone::Battlefield && o.zcc == self.source.1
    }

    /// Java argument `i` as a number (`new XEffect(2)`)
    pub fn arg_int(&self, i: usize) -> Option<i32> {
        self.args.get(i)?.trim().parse().ok()
    }

    /// Engine effects, as if they were the ability's own from here on: the same targets, source and controller.
    /// The way to reuse what cards.json can already say (`CreateTokenEffect`, `DestroyTargetEffect`…) inside a
    /// handler. Effects that need JSON-only data are built with `serde_json::from_value`.
    pub fn run(&self, game: &mut Game, d: &mut dyn Decider, effects: &[EffectDef]) {
        super::apply_each_effect(game, d, self.item, effects, self.legal, self.source);
    }

    /// `run` as if `seat` controlled the ability ("that player creates a token", "its controller draws")
    pub fn run_as(&self, game: &mut Game, d: &mut dyn Decider, effects: &[EffectDef], seat: Seat) {
        let item = StackItem { controller: seat, ..self.item.clone() };
        super::apply_each_effect(game, d, &item, effects, self.legal, self.source);
    }

    /// `run` with other target groups (a `FixedTarget`: the effect acts on these instead of the ability's targets)
    pub fn run_on(&self, game: &mut Game, d: &mut dyn Decider, effects: &[EffectDef], targets: Vec<Target>) {
        super::apply_each_effect(game, d, self.item, effects, &[targets], self.source);
    }
}

/// A one-shot effect: `OneShotEffect.apply`. None when the decider stopped (return at once: `?` on the rules' asks).
pub(crate) type EffectFn = fn(&mut Game, &mut dyn Decider, &EffectCx) -> Option<()>;
/// A `Condition.apply`, for the source's controller `you`
pub(crate) type ConditionFn = fn(&Game, Seat, ObjId) -> bool;
/// A `DynamicValue.calculate`, for the source's controller `you`, with the ability's X
pub(crate) type ValueFn = fn(&Game, Seat, ObjId, i32) -> i32;

#[allow(dead_code)]
pub(crate) enum Hook {
    Effect(EffectFn),
    Condition(ConditionFn),
    Value(ValueFn),
}

/// One Java class written in Rust
#[allow(dead_code)] // `tokens`, `effects` and `filters` are the transpiler's
pub(crate) struct Handler {
    /// `OuterCardClass.InnerClass` for a class a card file declares, else the class's name
    pub java: &'static str,
    pub hook: Hook,
    /// Java the transpiler translates for the handler, read from this declaration in the handler's file (so keep
    /// each a plain string literal): token classes (`"FrogLizardToken"`), effect expressions
    /// (`"new DiscardTargetEffect(1)"`) and filter expressions (`"StaticFilters.FILTER_PERMANENT_CREATURE"`, or a
    /// filter the card file declares). They reach the handler as `EffectCx::tokens`, `effects` and `filters`.
    pub tokens: &'static [&'static str],
    pub effects: &'static [&'static str],
    pub filters: &'static [&'static str],
}

#[allow(dead_code)]
impl Handler {
    const fn new(java: &'static str, hook: Hook) -> Handler {
        Handler { java, hook, tokens: &[], effects: &[], filters: &[] }
    }

    pub const fn effect(java: &'static str, f: EffectFn) -> Handler {
        Handler::new(java, Hook::Effect(f))
    }

    pub const fn condition(java: &'static str, f: ConditionFn) -> Handler {
        Handler::new(java, Hook::Condition(f))
    }

    pub const fn value(java: &'static str, f: ValueFn) -> Handler {
        Handler::new(java, Hook::Value(f))
    }

    pub const fn tokens(mut self, tokens: &'static [&'static str]) -> Handler {
        self.tokens = tokens;
        self
    }

    pub const fn effects(mut self, effects: &'static [&'static str]) -> Handler {
        self.effects = effects;
        self
    }

    pub const fn filters(mut self, filters: &'static [&'static str]) -> Handler {
        self.filters = filters;
        self
    }
}

include!(concat!(env!("OUT_DIR"), "/card_handlers.rs"));

/// The handler registered for a Java class (`OuterCardClass.InnerClass`)
pub(crate) fn find(java: &str) -> Option<&'static Handler> {
    static BY_NAME: OnceLock<HashMap<&'static str, &'static Handler>> = OnceLock::new();
    BY_NAME
        .get_or_init(|| {
            let mut m = HashMap::new();
            for h in ALL.iter().flat_map(|hs| hs.iter()) {
                assert!(m.insert(h.java, h).is_none(), "two card handlers for {}", h.java);
            }
            m
        })
        .get(java)
        .copied()
}

/// A cards.json reference to a handler, checked as the card loads: an unknown name, or a handler of another kind,
/// makes the card's JSON fail (and the card is left out of the engine)
macro_rules! handler_ref {
    ($name:ident, $hook:ident, $f:ty, $what:literal) => {
        #[derive(Clone, Copy)]
        pub struct $name(&'static str, $f);

        impl $name {
            pub fn java(&self) -> &'static str {
                self.0
            }

            pub(crate) fn f(&self) -> $f {
                self.1
            }
        }

        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                self.0 == other.0
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                let name = String::deserialize(d)?;
                match find(&name) {
                    Some(Handler { java, hook: Hook::$hook(f), .. }) => Ok($name(java, *f)),
                    Some(_) => Err(serde::de::Error::custom(format!("card handler {name} isn't {}", $what))),
                    None => Err(serde::de::Error::custom(format!("no card handler {name}"))),
                }
            }
        }
    };
}

handler_ref!(EffectHandler, Effect, EffectFn, "an effect");
handler_ref!(ConditionHandler, Condition, ConditionFn, "a condition");
handler_ref!(ValueHandler, Value, ValueFn, "a value");

#[cfg(test)]
mod tests {
    /// `PLAYMAT_CHECK_CARDS=<cards json>`: every card in it loads (a handler's card, transpiled on its own with
    /// `transpile.py --pool <names> --out <json>`)
    #[test]
    fn check_cards_load() {
        let Ok(path) = std::env::var("PLAYMAT_CHECK_CARDS") else { return };
        let text = std::fs::read_to_string(&path).unwrap();
        let (db, errors) = crate::card::CardDb::load_json(&text).unwrap();
        assert!(errors.is_empty(), "cards that don't load: {errors:#?}");
        eprintln!("{} cards load", db.len());
    }

    /// every handler's Java name is unique and well formed (`find` asserts uniqueness as it builds its map)
    #[test]
    fn handler_names_are_unique() {
        for h in super::ALL.iter().flat_map(|hs| hs.iter()) {
            assert!(h.java.chars().all(|c| c.is_ascii_alphanumeric() || c == '.'), "{}", h.java);
            // (by name: `HANDLERS` is a const, so its slices may be copies, not one place in memory)
            assert_eq!(super::find(h.java).map(|f| f.java), Some(h.java));
        }
    }
}
