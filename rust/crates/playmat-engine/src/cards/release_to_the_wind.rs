//! Release to the Wind: "Exile target nonland permanent. For as long as that card remains exiled, its owner may
//! cast it without paying its mana cost."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ReleaseToTheWind.ReleaseToTheWindEffect", apply)];

/// `exileAndPlayFromExile(…, TargetController.OWNER, Duration.Custom, withoutMana, !anyColor, onlyCast)`: the card
/// exiled, then its owner may cast it, free, while it stays there
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(t) = cx.obj(0) else { return Some(()) };
    if game.obj(t).zone != Zone::Battlefield {
        return Some(());
    }
    game.move_to(t, Zone::Exile);
    if game.obj(t).zone != Zone::Exile {
        return Some(());
    }
    let owner = game.obj(t).owner;
    let o = game.obj_mut(t);
    o.may_play = Some((owner, u32::MAX));
    o.may_play_end_step = false;
    o.may_play_while = None;
    o.play_free = true;
    o.cast_only = true;
    Some(())
}
