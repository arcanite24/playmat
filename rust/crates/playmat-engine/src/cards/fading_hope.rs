//! Fading Hope: "Return target creature to its owner's hand. If its mana value was 3 or less, scry 1."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("FadingHope.FadingHopeEffect", apply)];

/// The mana value as it was on the battlefield, then the return, then scry 1
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let small = game.obj(o).def.mana_value() <= 3;
    game.move_to(o, Zone::Hand);
    if small {
        cx.run(game, d, &[EffectDef::ScryEffect { amount: crate::card::Amount::Fixed(1) }]);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}
