//! Invasion of New Capenna // Holy Frazzle-Cannon: "Whenever equipped creature attacks, put a +1/+1 counter on that
//! creature and each other creature you control that shares a creature type with it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("InvasionOfNewCapenna.HolyFrazzleCannonEffect", apply)];

/// A +1/+1 counter on each creature you control that's the attacker or shares a creature type with it
/// (`MageObject.shareCreatureTypes`), in battlefield order
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let it = cx.obj(0)?;
    if game.obj(it).zone != Zone::Battlefield {
        return Some(());
    }
    let creatures: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == you && game.has_type(*o, CardType::Creature))
        .collect();
    for c in creatures {
        if c == it || share_creature_types(game, it, c) {
            put_counters(game, c, "+1/+1", 1, you);
        }
    }
    Some(())
}

/// `MageObject.shareCreatureTypes`: both creatures (or kindred) with a creature type, and one of `a`'s in `b`'s
/// (every creature type counts for any)
fn share_creature_types(game: &Game, a: ObjId, b: ObjId) -> bool {
    let kinds = |o: ObjId| game.has_type(o, CardType::Creature) || game.has_type(o, CardType::Kindred);
    if !kinds(a) || !kinds(b) {
        return false;
    }
    let all = |o: ObjId| game.obj(o).def.all_creature_types && game.obj(o).creature_types.is_none();
    let types = |o: ObjId| -> Vec<String> {
        let obj = game.obj(o);
        let own = obj.creature_types.clone().unwrap_or_else(|| obj.def.subtypes.clone());
        own.iter()
            .map(String::as_str)
            .chain(game.added_subtypes(o))
            .filter(|t| !crate::card::NON_CREATURE_TYPES.contains(t))
            .map(str::to_string)
            .collect()
    };
    let (ta, tb) = (types(a), types(b));
    let any_a = all(a) || !ta.is_empty();
    let any_b = all(b) || !tb.is_empty();
    if !any_a || !any_b {
        return false;
    }
    all(a) || ta.iter().any(|t| game.has_subtype(b, t))
}
