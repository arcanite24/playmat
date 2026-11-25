//! Archmage's Newt: "Whenever Archmage's Newt deals combat damage to a player, target instant or sorcery card in your
//! graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost. That card gains
//! flashback {0} until end of turn instead if Archmage's Newt is saddled."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ArchmagesNewt.ArchmagesNewtEffect", apply)];

/// `GainFlashbackTargetEffect`'s flashback (an ability after the card's own), for {0} when the Newt is saddled as
/// the effect starts (`init`: `SaddledCondition`)
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let card = cx.obj(0)?;
    if game.obj(card).zone != Zone::Graveyard {
        return Some(());
    }
    let free = game.condition_holds(Some(&crate::card::Condition::SourceSaddled), cx.controller(), cx.source_id());
    let turn = game.turn;
    let o = game.obj_mut(card);
    o.flashback_turn = Some(turn);
    o.flashback_free = free;
    Some(())
}
