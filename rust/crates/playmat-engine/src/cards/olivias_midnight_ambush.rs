//! Olivia's Midnight Ambush: "Target creature gets -2/-2 until end of turn. If it's night, that creature gets -13/-13
//! until end of turn instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("OliviasMidnightAmbush.OliviasMidnightAmbushEffect", apply)];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(t) = cx.obj(0) else { return Some(()) };
    if game.obj(t).zone != Zone::Battlefield {
        return Some(());
    }
    let n = if game.day == Some(false) { -13 } else { -2 };
    boost(game, t, n, n, crate::card::Duration::EndOfTurn);
    Some(())
}
