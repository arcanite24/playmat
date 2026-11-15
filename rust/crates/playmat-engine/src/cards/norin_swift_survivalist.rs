//! Norin, Swift Survivalist: "Norin, Swift Survivalist can't block. Whenever a creature you control becomes blocked,
//! you may exile it. You may play that card from exile this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("NorinSwiftSurvivalist.NorinSwiftSurvivalistEffect", apply)];

/// the blocked creature (still on the battlefield) exiled; its card, if it's in exile, playable by you this turn
/// (`makeCardPlayable(…, Duration.EndOfTurn)`)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(o) = cx.obj(0).filter(|o| game.obj(*o).zone == Zone::Battlefield) else { return Some(()) };
    let token = game.obj(o).token;
    game.move_to(o, Zone::Exile);
    if !token && game.obj(o).zone == Zone::Exile {
        let turn = game.turn;
        game.obj_mut(o).may_play = Some((you, turn));
    }
    Some(())
}
