//! Cryptic Coat: "When Cryptic Coat enters, cloak the top card of your library, then attach Cryptic Coat to it.
//! Equipped creature gets +1/+0 and can't be blocked. {1}{U}: Return Cryptic Coat to its owner's hand."

use super::*;
use crate::card::Amount;

pub const HANDLERS: &[Handler] = &[Handler::effect("CrypticCoat.CrypticCoatEffect", apply)];

/// `ManifestEffect.doManifestCards(top card, cloak)`, then `AttachEffect` with the cloaked permanent as its target
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(top) = game.player(cx.controller()).library.first().copied() else { return Some(()) };
    cx.run_on(game, d, &[EffectDef::ManifestEffect { amount: Amount::Fixed(1), cloak: true }], Vec::new());
    if d.stopped() {
        return None;
    }
    if game.obj(top).zone != Zone::Battlefield {
        return Some(());
    }
    cx.run_on(game, d, &[EffectDef::AttachEffect], vec![Target::Obj(top)]);
    Some(())
}
