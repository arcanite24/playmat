//! Presumed Dead: "Until end of turn, target creature gets +2/+0 and gains "When this creature dies, return it to the
//! battlefield under its owner's control and suspect it.""

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("PresumedDead.PresumedDeadEffect", apply)];

/// the dies trigger's card (`SetTargetPointer.CARD`), still in the graveyard: onto the battlefield under its owner's
/// control, then the permanent it became is suspected
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(card) = cx.obj(0) else { return Some(()) };
    if game.obj(card).zone != Zone::Graveyard {
        return Some(());
    }
    let owner = game.obj(card).owner;
    put_onto_battlefield(game, d, card, owner);
    if d.stopped() {
        return None;
    }
    if game.obj(card).zone == Zone::Battlefield {
        game.obj_mut(card).set_suspected(true);
    }
    Some(())
}
