//! Ritual of the Returned: "Exile target creature card from your graveyard. Create a black Zombie creature token.
//! Its power is equal to that card's power and its toughness is equal to that card's toughness."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RitualOfTheReturned.RitualOfTheReturnedExileEffect", apply)
    .tokens(&["RitualOfTheReturnedZombieToken"])];

/// the target, still in the graveyard, is exiled; then a Zombie with its printed power and toughness
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(card) = cx.obj(0) else { return Some(()) };
    if game.obj(card).zone != Zone::Graveyard {
        return Some(());
    }
    let (power, toughness) = (game.obj(card).def.power, game.obj(card).def.toughness);
    game.move_to(card, Zone::Exile);
    let mut token = cx.tokens[0].clone();
    token.power = power;
    token.toughness = toughness;
    cx.create_tokens(game, d, &token, 1);
    if d.stopped() {
        return None;
    }
    Some(())
}
