//! Hidetsugu's Second Rite: "If target player has exactly 10 life, Hidetsugu's Second Rite deals 10 damage to that
//! player."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("HidetsugusSecondRite.HidetsugusSecondRiteEffect", apply)];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = cx.player(0) else { return Some(()) };
    if game.player(p).life == 10 {
        deal_damage(game, cx.source_id(), Target::Player(p), 10, false);
    }
    Some(())
}
