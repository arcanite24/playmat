//! Hiveheart Shaman: "Whenever Hiveheart Shaman attacks, you may search your library for a basic land card that
//! doesn't share a land type with a land you control, put that card onto the battlefield, then shuffle. {5}{G}:
//! Create a 1/1 green Insect creature token. Put X +1/+1 counters on it, where X is the number of basic land types
//! among lands you control. Activate only as a sorcery."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::predicate("HiveheartShaman.HiveheartShamanPredicate", unshared_basic),
    Handler::effect("HiveheartShaman.HiveheartShamanEffect", apply).tokens(&["InsectToken"]),
];

/// `SubTypeSet.BasicLandType` and `SubTypeSet.NonBasicLandType`
const LAND_TYPES: [&str; 17] = [
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
];

const BASIC: [&str; 5] = ["Plains", "Island", "Swamp", "Mountain", "Forest"];

fn lands(game: &Game, you: Seat) -> Vec<ObjId> {
    game.battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == you && game.has_type(*o, CardType::Land))
        .collect()
}

/// a basic land card with no land type any land you control has
fn unshared_basic(game: &Game, you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    if !game.obj(id).def.supertypes.iter().any(|s| s == "BASIC") || !game.has_type(id, CardType::Land) {
        return false;
    }
    let mine = lands(game, you);
    !LAND_TYPES.iter().any(|t| game.has_subtype(id, t) && mine.iter().any(|l| game.has_subtype(*l, t)))
}

/// The Insect, then (counted after it's made) one +1/+1 counter on it for each basic land type among your lands
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let before = game.objs.len();
    cx.create_tokens(game, d, &cx.tokens[0], 1);
    if d.stopped() {
        return None;
    }
    let mine = lands(game, you);
    let domain = BASIC.iter().filter(|t| mine.iter().any(|l| game.has_subtype(*l, t))).count() as i32;
    if domain < 1 {
        return Some(());
    }
    let made: Vec<ObjId> = (before..game.objs.len())
        .map(|i| ObjId(i as u32))
        .filter(|o| game.obj(*o).token && game.obj(*o).zone == Zone::Battlefield)
        .collect();
    for t in made {
        add_counters_by(game, t, "+1/+1", domain, Some(you));
    }
    Some(())
}
