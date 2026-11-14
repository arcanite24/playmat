//! Orphans of the Wheat: "Whenever Orphans of the Wheat attacks, tap any number of untapped creatures you control.
//! Orphans of the Wheat gets +1/+1 until end of turn for each creature tapped this way."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("OrphansOfTheWheat.OrphansOfTheWheatEffect", apply)];

/// `TargetPermanent(0, MAX, untapped creatures you control, notTarget)` with one `Player.choose`, each tapped, then
/// the boost for as many as were chosen
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let possible: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == you && !game.obj(*o).tapped && game.has_type(*o, CardType::Creature))
        .collect();
    // (asked even with nothing to choose)
    let max = possible.len().max(1);
    let chosen = choose_cards(game, d, you, &possible, &possible, 0, max, &item_rule(game, cx.item))?;
    let chosen: Vec<ObjId> = chosen.into_iter().filter(|o| game.obj(*o).zone == Zone::Battlefield).collect();
    for o in &chosen {
        tap(game, *o);
    }
    if chosen.is_empty() {
        return None;
    }
    let n = chosen.len() as i32;
    let me = cx.source_id();
    if game.obj(me).zone == Zone::Battlefield {
        boost(game, me, n, n, Duration::EndOfTurn);
    }
    Some(())
}
