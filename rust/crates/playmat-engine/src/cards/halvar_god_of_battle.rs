//! Halvar, God of Battle // Sword of the Realms. Halvar: "At the beginning of each combat, you may attach target Aura
//! or Equipment attached to a creature you control to target creature you control." Sword of the Realms: "Whenever
//! equipped creature dies, return it to its owner's hand."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::predicate("HalvarGodOfBattle.HalvarGodOfBattlePredicate", attached_to_yours),
    Handler::effect("HalvarGodOfBattle.SwordOfTheRealmsEffect", return_it),
];

/// `HalvarGodOfBattlePredicate`: the object is attached to a permanent the filter's player controls
fn attached_to_yours(game: &Game, you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    game.attached(id).is_some_and(|to| game.obj(to).controller == you)
}

/// `SwordOfTheRealmsEffect`: the creature that died (`getValue("attachedTo")`), while it's still the card that went
/// to the graveyard (its zone change count), to its owner's hand
fn return_it(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if let Some(card) = cx.obj(0) {
        game.move_to(card, Zone::Hand);
    }
    Some(())
}
