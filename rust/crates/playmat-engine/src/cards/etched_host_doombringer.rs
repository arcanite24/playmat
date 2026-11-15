//! Etched Host Doombringer: "When Etched Host Doombringer enters, choose one — • Target opponent loses 2 life and you
//! gain 2 life. • Choose target battle. If an opponent protects it, remove three defense counters from it.
//! Otherwise, put three defense counters on it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("EtchedHostDoombringer.EtchedHostDoombringerEffect", apply)];

/// The battle still there: protected by an opponent (`ProtectedByOpponentPredicate`), three defense counters off it;
/// else three on it
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let id = cx.obj(0)?;
    if game.obj(id).zone != Zone::Battlefield {
        return Some(());
    }
    let by_opponent = game.obj(id).protector.is_some_and(|p| Game::whose("opponent", you, p));
    if !by_opponent {
        add_counters_by(game, id, "defense", 3, Some(you));
        return Some(());
    }
    let o = game.obj_mut(id);
    o.add_counters("defense", -3);
    o.counters.retain(|(n, k)| n != "defense" || *k > 0);
    Some(())
}
