//! Fatal Grudge: "As an additional cost to cast this spell, sacrifice a nonland permanent. Each opponent chooses a
//! permanent they control that shares a card type with the sacrificed permanent and sacrifices it. Draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("FatalGrudge.FatalGrudgeEffect", apply)];

/// The sacrificed permanent's card types (as it last was, when the engine kept it; else its card's); each opponent
/// chooses one of theirs sharing one (`TargetSacrifice`, required) and sacrifices it
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut types: Vec<CardType> = Vec::new();
    for s in &cx.item.sacrificed {
        let o = game.obj(*s);
        types.extend(o.lki_def.as_ref().unwrap_or(&o.def).types.iter().copied());
    }
    let n = game.seats();
    let opponents: Vec<Seat> = (0..n).map(|k| (you + k) % n).filter(|p| *p != you && !game.player(*p).lost).collect();
    let rule = item_rule(game, cx.item);
    for p in opponents {
        let possible: Vec<ObjId> = game
            .battlefield
            .iter()
            .copied()
            .filter(|o| game.obj(*o).controller == p && types.iter().any(|t| game.has_type(*o, *t)))
            .collect();
        if possible.is_empty() {
            continue;
        }
        if let Some(&o) = choose_cards(game, d, p, &possible, &possible, 1, 1, &rule)?.first() {
            if game.obj(o).zone == Zone::Battlefield {
                sacrifice(game, o);
            }
        }
    }
    Some(())
}
