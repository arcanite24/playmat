//! Exhilarating Elocution: "Put two +1/+1 counters on target creature you control. Other creatures you control get
//! +1/+1 until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ExhilaratingElocution.ExhilaratingElocutionEffect", apply)];

/// The counters, then +1/+1 until end of turn on each other creature you control now (battlefield order)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(t) = cx.obj(0) else { return Some(()) };
    if game.obj(t).zone != Zone::Battlefield {
        return Some(());
    }
    let you = cx.controller();
    add_counters_by(game, t, "+1/+1", 2, Some(you));
    let others: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| *o != t && game.obj(*o).controller == you && game.has_type(*o, CardType::Creature))
        .collect();
    for o in others {
        boost(game, o, 1, 1, crate::card::Duration::EndOfTurn);
    }
    Some(())
}
