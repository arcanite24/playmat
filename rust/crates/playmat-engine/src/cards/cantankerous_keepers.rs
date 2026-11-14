//! Cantankerous Keepers: "When this creature enters, mill four cards, then put all Elf cards from among them into your
//! hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("CantankerousKeepers.CantankerousKeepersEffect", apply)];

/// The engine's mill; the milled cards still in the graveyard that are Elves go to your hand
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let top: Vec<ObjId> = game.player(you).library.iter().take(4).copied().collect();
    cx.run(game, d, &[EffectDef::MillCardsControllerEffect { amount: Amount::Fixed(4) }]);
    if d.stopped() {
        return None;
    }
    for c in top {
        if game.obj(c).zone == Zone::Graveyard && game.has_subtype(c, "Elf") {
            game.move_to(c, Zone::Hand);
        }
    }
    Some(())
}
