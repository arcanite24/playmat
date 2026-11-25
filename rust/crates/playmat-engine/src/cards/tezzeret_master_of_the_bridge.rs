//! Tezzeret, Master of the Bridge: "−8: Exile the top ten cards of your library. Put all artifact cards from among
//! them onto the battlefield."

use super::*;
use crate::card::PutCards;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("TezzeretMasterOfTheBridge.TezzeretMasterOfTheBridgeEffect2", apply)];

/// the top ten exiled (in library order), then the artifact cards among them that are still in exile onto the
/// battlefield together
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let top: Vec<ObjId> = game.player(you).library.iter().take(10).copied().collect();
    if !put_cards(game, d, you, &top, PutCards::Exiled) {
        return None;
    }
    let artifacts: Vec<ObjId> = top
        .into_iter()
        .filter(|c| game.obj(*c).zone == Zone::Exile && game.obj(*c).is(CardType::Artifact))
        .collect();
    put_cards(game, d, you, &artifacts, PutCards::Battlefield).then_some(())
}
