//! Mindreaver: "Heroic — Whenever you cast a spell that targets Mindreaver, exile the top three cards of target
//! player's library. {U}{U}, Sacrifice Mindreaver: Counter target spell with the same name as a card exiled with
//! Mindreaver."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("Mindreaver.MindreaverExileEffect", exile),
    Handler::target("Mindreaver.MindreaverTarget", "new TargetSpell()", same_name),
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

/// `canTarget`: a spell with the name of a card in Mindreaver's exile zone (`CardUtil.haveSameNames`). The zone is
/// the Mindreaver on the battlefield, or (once it's sacrificed for the ability) the one it was
fn same_name(game: &Game, _you: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    let Some(src) = source else { return false };
    let o = game.obj(src);
    let zcc = if o.zone == Zone::Battlefield { o.zcc } else { o.zcc.saturating_sub(1) };
    let name = game.obj(id).name();
    !name.is_empty()
        && game.exile.iter().any(|c| game.obj(*c).exiled_with == Some((src, zcc)) && game.obj(*c).name() == name)
}
