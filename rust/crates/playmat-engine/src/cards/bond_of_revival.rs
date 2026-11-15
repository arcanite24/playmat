//! Bond of Revival: "Return target creature card from your graveyard to the battlefield. It gains haste until your
//! next turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BondOfRevival.BondOfRevivalEffect", apply)];

/// The card onto the battlefield under your control; the permanent it became gains haste until your next turn
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(c) = cx.obj(0) else { return Some(()) };
    if game.obj(c).zone != Zone::Graveyard {
        return Some(());
    }
    let you = cx.controller();
    put_onto_battlefield(game, d, c, you);
    if d.stopped() {
        return None;
    }
    if game.obj(c).zone == Zone::Battlefield {
        let zcc = game.obj(c).zcc;
        Arc::make_mut(&mut game.grants).push(Grant {
            target: c,
            zcc,
            ability: AbilityDef::HasteAbility,
            until_end_of_turn: false,
            until_turn_of: Some(you),
        });
    }
    Some(())
}
