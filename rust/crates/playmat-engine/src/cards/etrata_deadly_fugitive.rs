//! Etrata, Deadly Fugitive: "Face-down creatures you control have "{2}{U}{B}: Turn this creature face up. If you
//! can't, exile it, then you may cast the exiled card without paying its mana cost." Whenever an Assassin you control
//! deals combat damage to an opponent, cloak the top card of that player's library."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("EtrataDeadlyFugitive.EtrataCloakEffect", cloak),
    Handler::effect("EtrataDeadlyFugitive.EtrataFlipEffect", flip),
];

/// `ManifestEffect.doManifestCards(…, true)` of the damaged player's top card, under your control
fn cloak(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let opponent = cx.player(0)?;
    let top = game.player(opponent).library.first().copied()?;
    manifest(game, d, cx.controller(), &[top], true);
    if d.stopped() {
        return None;
    }
    Some(())
}

/// `turnFaceUp`: a permanent card under it turns face up; an instant or sorcery card is revealed and stays face down
/// (701.34g), so the card is exiled and may be cast without paying its mana cost
/// (`castSpellWithAttributesForFree`)
fn flip(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let id = cx.source_id();
    if game.obj(id).zone != Zone::Battlefield {
        return Some(());
    }
    let card = game.obj(id).face_up.clone()?;
    if card.is_permanent_card() {
        turn_face_up(game, id, false);
        return Some(());
    }
    let zcc = game.obj(id).zcc;
    game.revealed.push(crate::state::Revealed { source: id, zcc, paren: false, cards: vec![(id, Zone::Battlefield)] });
    game.move_to(id, Zone::Exile);
    if game.obj(id).zone != Zone::Exile {
        return Some(());
    }
    cast_one_for_free_from(game, d, you, id, &crate::card::Filter::default(), &[id]).then_some(())
}
