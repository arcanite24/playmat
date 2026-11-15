//! Rescuer Sphinx: "Flying. As Rescuer Sphinx enters, you may return a nonland permanent you control to its
//! owner's hand. If you do, Rescuer Sphinx enters with a +1/+1 counter on it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RescuerSphinx.RescuerSphinxEffect", apply)];

/// `chooseUse`, then `player.choose(TargetPermanent(0, 1, nonland permanent you control, true))`; the chosen one
/// returned to its owner's hand, and then a +1/+1 counter on the entering Sphinx (`AddCountersSourceEffect`). The
/// Sphinx itself is still entering in Java, so it's no choice.
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let me = cx.source_id();
    if !choose_use(game, d, you, "Return a nonland permanent you control to your hand?")? {
        return Some(());
    }
    let can: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| *o != me && game.obj(*o).controller == you && !game.has_type(*o, CardType::Land))
        .collect();
    let chosen = choose_cards_rounds(game, d, you, &can, 0, 1)?;
    let Some(p) = chosen.first().copied() else { return Some(()) };
    if game.obj(p).zone != Zone::Battlefield {
        return Some(());
    }
    game.move_to(p, Zone::Hand);
    if game.obj(p).zone != Zone::Hand {
        return Some(());
    }
    if game.obj(me).zone == Zone::Battlefield && game.obj(me).zcc == cx.source.1 {
        add_counters_by(game, me, "+1/+1", 1, Some(you));
    }
    Some(())
}
