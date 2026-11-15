//! Pharika, God of Affliction: "Indestructible. As long as your devotion to black and green is less than seven,
//! Pharika isn't a creature. {B}{G}: Exile target creature card from a graveyard. Its owner creates a 1/1 black and
//! green Snake enchantment creature token with deathtouch."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("PharikaGodOfAffliction.PharikaExileEffect", apply).tokens(&["PharikaSnakeToken"])];

/// the target card exiled (`controller.moveCards`) while it's still in a graveyard, then its owner creates the Snake
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let card = cx.obj(0)?;
    if game.obj(card).zone == Zone::Graveyard && !put_cards(game, d, cx.controller(), &[card], crate::card::PutCards::Exiled)
    {
        return None;
    }
    let owner = game.obj(card).owner;
    let e = EffectDef::CreateTokenEffect {
        token: cx.tokens[0].clone(),
        amount: crate::card::Amount::Fixed(1),
        tapped_and_attacking: false,
        tapped: false,
        attacking: false,
        sacrifice_at_end_of_combat: false,
        sacrifice_at_next_end_step: false,
        haste_until_end_of_turn: false,
        also: None,
        attach_source: None,
    };
    cx.run_as(game, d, &[e], owner);
    Some(())
}
