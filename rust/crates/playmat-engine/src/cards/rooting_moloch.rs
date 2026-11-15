//! Rooting Moloch: "When Rooting Moloch enters, exile target card with a cycling ability from your graveyard. Until
//! the end of your next turn, you may play that card. Cycling {2}"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RootingMoloch.RootingMolochEffect", apply)];

/// The target card exiled; you may play it until your next turn's end step begins (`RootingMolochMayPlayEffect`'s
/// `isInactive`: a later turn's `END_TURN` step with you active; this turn's too when it's an opponent's)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(c) = cx.obj(0) else { return Some(()) };
    game.move_to(c, Zone::Exile);
    if game.obj(c).zone != Zone::Exile {
        return Some(());
    }
    let until = if game.active == Some(you) { game.turn + game.seats() as u32 } else { game.turn + 1 };
    let o = game.obj_mut(c);
    o.may_play = Some((you, until));
    o.may_play_end_step = true;
    o.may_play_while = None;
    o.cast_only = false;
    Some(())
}
