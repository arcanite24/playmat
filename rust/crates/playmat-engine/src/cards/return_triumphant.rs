//! Return Triumphant: "Return target creature card with mana value 3 or less from your graveyard to the
//! battlefield. Create a Young Hero Role token attached to it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ReturnTriumphant.ReturnTriumphantAbility", apply)
    .effects(&["new CreateRoleAttachedTargetEffect(RoleType.YOUNG_HERO)"])];

/// `moveCards` onto the battlefield, then the Role attached to the permanent it became
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(c) = cx.obj(0) else { return Some(()) };
    if game.obj(c).zone != Zone::Graveyard {
        return Some(());
    }
    if !put_cards(game, d, cx.controller(), &[c], crate::card::PutCards::Battlefield) {
        return None;
    }
    if game.obj(c).zone != Zone::Battlefield {
        return Some(());
    }
    cx.run_on(game, d, &cx.effects[..1], vec![Target::Obj(c)]);
    if d.stopped() {
        return None;
    }
    Some(())
}
