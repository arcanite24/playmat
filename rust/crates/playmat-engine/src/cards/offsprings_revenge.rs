//! Offspring's Revenge: "At the beginning of combat on your turn, exile target red, white, or black creature card
//! from your graveyard. Create a token that's a copy of that card, except it's 1/1. It gains haste until your next
//! turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("OffspringsRevenge.OffspringsRevengeEffect", apply)];

/// The target card exiled, then a 1/1 token copy of it (`CreateTokenCopyTargetEffect(…, 1, 1, false)` pointed at
/// the card in exile), each token with haste until your next turn
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(card) = cx.obj(0) else { return Some(()) };
    if game.obj(card).zone != Zone::Graveyard {
        return Some(());
    }
    game.move_to(card, Zone::Exile);
    if game.obj(card).zone != Zone::Exile {
        return Some(());
    }
    let made = token_copy_modified(game, d, card, you, 1, &|def: &mut CardDef| {
        def.remove_pt_cda();
        def.power = 1;
        def.toughness = 1;
    });
    if d.stopped() {
        return None;
    }
    for t in made {
        if game.obj(t).zone != Zone::Battlefield {
            continue;
        }
        let zcc = game.obj(t).zcc;
        Arc::make_mut(&mut game.grants).push(crate::state::Grant {
            target: t,
            zcc,
            ability: AbilityDef::HasteAbility,
            until_end_of_turn: false,
            until_turn_of: Some(you),
        });
    }
    Some(())
}
