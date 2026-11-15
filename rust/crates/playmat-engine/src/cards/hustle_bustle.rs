//! Hustle // Bustle: "Creatures you control get +2/+2 and gain trample until end of turn. You may turn a face-down
//! creature you control face up."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("HustleBustle.HustleBustleEffect", apply)];

/// up to one face-down creature you control (`TargetPermanent(0, 1, …, true)` with `choose`), turned face up
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let possible: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|p| {
            let o = game.obj(*p);
            o.controller == you && o.face_down && game.has_type(*p, CardType::Creature)
        })
        .collect();
    if possible.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &possible, &possible, 0, 1, &rule)?;
    if let Some(p) = chosen.first().copied() {
        if game.obj(p).zone == Zone::Battlefield && game.obj(p).face_down {
            turn_face_up(game, p, false);
        }
    }
    Some(())
}
