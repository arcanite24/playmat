//! Mindreaver: "Heroic — Whenever you cast a spell that targets Mindreaver, exile the top three cards of target
//! player's library. {U}{U}, Sacrifice Mindreaver: Counter target spell with the same name as a card exiled with
//! Mindreaver."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("Mindreaver.MindreaverExileEffect", exile),
    Handler::target("Mindreaver.MindreaverTarget", "new TargetSpell()", any_spell),
];

/// The target player's top three cards exiled in this permanent's exile zone
fn exile(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = cx.player(0) else { return Some(()) };
    let top: Vec<ObjId> = game.player(p).library.iter().take(3).copied().collect();
    let zone = (cx.source_id(), game.obj(cx.source_id()).zcc);
    for c in top {
        game.move_to(c, Zone::Exile);
        if game.obj(c).zone == Zone::Exile {
            game.obj_mut(c).exiled_with = Some(zone);
        }
    }
    Some(())
}

/// `MindreaverTarget.canTarget` asks for a card of the same name in Mindreaver's exile zone, but the class doesn't
/// override `copy()`: the permanent's copy of the ability gets `TargetSpell.copy()`, a plain `TargetSpell` with the
/// filter's text and nothing else, so in Java any spell can be targeted (and countered).
fn any_spell(_game: &Game, _you: Seat, _source: Option<ObjId>, _id: ObjId) -> bool {
    true
}
