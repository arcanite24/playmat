//! Cemetery Recruitment: "Return target creature card from your graveyard to your hand. If it's a Zombie card, draw
//! a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("CemeteryRecruitment.CemeteryRecruitmentEffect", apply)];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(c) = cx.obj(0) else { return Some(()) };
    if game.obj(c).zone != Zone::Graveyard {
        return Some(());
    }
    game.move_to(c, Zone::Hand);
    if game.obj(c).zone == Zone::Hand && game.has_subtype(c, "Zombie") {
        draw(game, cx.controller(), 1);
    }
    Some(())
}
