//! Induced Amnesia: "When Induced Amnesia enters, target player exiles all the cards in their hand face down, then
//! draws that many cards." and "When Induced Amnesia is put into a graveyard from the battlefield, return the exiled
//! cards to their owner's hand."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("InducedAmnesia.InducedAmnesiaExileEffect", exile),
    Handler::effect("InducedAmnesia.InducedAmnesiaReturnEffect", give_back),
];

/// The target player's whole hand exiled together into this object's exile zone (its zone-change count now), those
/// now in exile turned face down; then they draw as many as were in hand
fn exile(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let p = cx.player(0)?;
    let hand = game.player(p).hand.clone();
    if hand.is_empty() {
        return Some(());
    }
    let src = cx.source_id();
    let zone = (src, game.obj(src).zcc);
    for c in &hand {
        game.move_to(*c, Zone::Exile);
    }
    for c in &hand {
        if game.obj(*c).zone == Zone::Exile {
            let o = game.obj_mut(*c);
            o.exiled_with = Some(zone);
            o.face_down = true;
        }
    }
    draw(game, p, hand.len());
    Some(())
}

/// The cards in the exile zone of the permanent it was (its zone-change count before it went to the graveyard),
/// each owner's together, to their owners' hands
fn give_back(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let src = cx.source_id();
    let Some(zcc) = game.obj(src).zcc.checked_sub(1) else { return Some(()) };
    let exiled: Vec<ObjId> =
        game.exile.iter().copied().filter(|c| game.obj(*c).exiled_with == Some((src, zcc))).collect();
    for owner in 0..game.seats() {
        for c in exiled.iter().copied() {
            if game.obj(c).owner == owner && game.obj(c).zone == Zone::Exile {
                game.obj_mut(c).face_down = false;
                game.move_to(c, Zone::Hand);
            }
        }
    }
    Some(())
}
