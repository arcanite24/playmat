//! Rundvelt Hordemaster: "Whenever this creature or another Goblin you control dies, exile the top card of your
//! library. If it's a Goblin creature card, you may cast that card until the end of your next turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RundveltHordemaster.RundveltHordemasterEffect", apply)];

/// `CardUtil.makeCardPlayable(…, useCastSpellOnly = true, UntilEndOfYourNextTurn, anyColor = false)`
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(c) = game.player(you).library.first().copied() else { return Some(()) };
    game.move_to(c, Zone::Exile);
    if game.obj(c).zone != Zone::Exile {
        return Some(());
    }
    if game.has_type(c, CardType::Creature) && game.has_subtype(c, "Goblin") {
        // through the end of your next turn (`ExileTopXMayPlayUntilEffect`'s reckoning)
        let until = if game.active == Some(you) { game.turn + game.seats() as u32 } else { game.turn + 1 };
        let o = game.obj_mut(c);
        o.may_play = Some((you, until));
        o.cast_only = true;
    }
    Some(())
}
