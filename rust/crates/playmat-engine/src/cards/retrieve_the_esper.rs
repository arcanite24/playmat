//! Retrieve the Esper: "Create a 3/3 blue Robot Warrior artifact creature token. Then if this spell was cast from a
//! graveyard, put two +1/+1 counters on that token. Flashback {5}{U}"

use super::*;
use crate::card::Condition;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("RetrieveTheEsper.RetrieveTheEsperEffect", apply).tokens(&["RobotBlueToken"])];

/// the Robot, then (`CastFromGraveyardSourceCondition`) two +1/+1 counters on the token it made
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let before = game.battlefield.clone();
    cx.create_tokens(game, d, &cx.tokens[0], 1);
    if d.stopped() {
        return None;
    }
    let you = cx.controller();
    if !game.condition_holds(Some(&Condition::CastFrom { zone: "graveyard".into() }), you, cx.source_id()) {
        return Some(());
    }
    let made: Vec<ObjId> = game.battlefield.iter().copied().filter(|o| !before.contains(o) && game.obj(*o).token).collect();
    for t in made {
        put_counters(game, t, "+1/+1", 2, you);
    }
    Some(())
}
