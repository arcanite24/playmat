//! Soul Separator: "{5}, {T}, Sacrifice Soul Separator: Exile target creature card from your graveyard. Create a
//! token that's a copy of that card, except it's 1/1, it's a Spirit in addition to its other types, and it has
//! flying. Create a black Zombie creature token with power equal to that card's power and toughness equal to that
//! card's toughness."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("SoulSeparator.SoulSeparatorEffect", apply).tokens(&["ZombieStarToken"])];

/// the target still in your graveyard: exile it, then a Zombie with its power and toughness (the card's own)
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
