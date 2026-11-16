//! Skyclave Aerialist // Skyclave Invader: "When this creature transforms into Skyclave Invader, look at the top card
//! of your library. If it's a land card, you may put it onto the battlefield. If you don't put the card onto the
//! battlefield, put it into your hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SkyclaveAerialist.SkyclaveInvaderEffect", apply)];

/// the top card looked at; a land: "Put … onto the battlefield or into your hand?" (Battlefield is yes)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let card = game.player(you).library.first().copied()?;
    let (source, zcc) = (cx.source_id(), game.obj(cx.source_id()).zcc);
    game.looked_at.push(crate::state::LookedAt { viewer: you, source, zcc, bare: false, cards: vec![card] });
    let land = game.obj(card).def.is(CardType::Land);
    let q = format!("Put {} onto the battlefield or into your hand?", game.obj(card).name());
    if land && choose_use(game, d, you, &q)? {
        put_onto_battlefield(game, d, card, you);
        return (!d.stopped()).then_some(());
    }
    game.move_to(card, Zone::Hand);
    Some(())
}
