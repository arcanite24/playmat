//! Oketra's Last Mercy: "Your life total becomes your starting life total. Lands you control don't untap during your
//! next untap step."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("OketrasLastMercy.OketrasLastMercyEffect", apply)];

/// `PlayerImpl.setLife`: the difference gained or lost
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let (life, start) = (game.player(you).life, game.starting_life);
    if start > life {
        gain_life(game, you, start - life);
    } else if start < life {
        lose_life(game, you, life - start);
    }
    Some(())
}
