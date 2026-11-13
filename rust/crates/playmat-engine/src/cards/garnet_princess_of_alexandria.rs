//! Garnet, Princess of Alexandria: "Whenever Garnet attacks, you may remove a lore counter from each of any number
//! of Sagas you control. Put a +1/+1 counter on Garnet for each lore counter removed this way."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("GarnetPrincessOfAlexandria.GarnetPrincessOfAlexandriaEffect", apply)];

/// `TargetPermanent(0, any, Sagas you control with a lore counter)` with `Player.choose` (asked even with none),
/// one lore counter off each chosen, then that many +1/+1 counters on Garnet while it's still the permanent
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let sagas: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == you && game.has_subtype(*o, "Saga") && game.obj(*o).counter("lore") > 0)
        .collect();
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &sagas, &sagas, 0, usize::MAX, &rule)?;
    let mut n = 0;
    for s in chosen {
        if game.obj(s).zone == Zone::Battlefield && game.obj(s).counter("lore") > 0 {
            game.obj_mut(s).add_counters("lore", -1);
            n += 1;
        }
    }
    if n > 0 && cx.source_on_battlefield(game) {
        put_counters(game, cx.source_id(), "+1/+1", n, you);
    }
    Some(())
}
