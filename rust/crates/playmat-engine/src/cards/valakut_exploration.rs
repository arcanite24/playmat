//! Valakut Exploration: "Landfall — Whenever a land you control enters, exile the top card of your library. You may
//! play that card for as long as it remains exiled. At the beginning of your end step, if there are cards exiled
//! with Valakut Exploration, put them into their owner's graveyard, then Valakut Exploration deals that much damage
//! to each opponent."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::condition("ValakutExploration.ValakutExplorationCondition", any_exiled),
    Handler::effect("ValakutExploration.ValakutExplorationExileEffect", exile),
    Handler::effect("ValakutExploration.ValakutExplorationDamageEffect", damage),
];

fn exiled_with(game: &Game, zone: (ObjId, u32)) -> Vec<ObjId> {
    game.exile.iter().copied().filter(|c| game.obj(*c).exiled_with == Some(zone)).collect()
}

/// cards in the exile zone of the source as it is now
fn any_exiled(game: &Game, _you: Seat, source: ObjId) -> bool {
    !exiled_with(game, (source, game.obj(source).zcc)).is_empty()
}

/// The top card goes to the source's exile zone (its stack moment's), playable by you for the rest of the game
fn exile(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(card) = game.player(you).library.first().copied() else { return Some(()) };
    game.move_to(card, Zone::Exile);
    if game.obj(card).zone == Zone::Exile {
        let o = game.obj_mut(card);
        o.exiled_with = Some(cx.source);
        o.may_play = Some((you, u32::MAX));
    }
    Some(())
}

/// The cards of the source's exile zone go to their owners' graveyards; that many damage to each opponent
fn damage(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let cards = exiled_with(game, cx.source);
    if cards.is_empty() {
        return Some(());
    }
    for c in &cards {
        game.move_to(*c, Zone::Graveyard);
    }
    let n = cards.len() as i32;
    let you = cx.controller();
    for p in 0..game.seats() {
        if p != you && !game.player(p).lost {
            deal_damage(game, cx.source_id(), Target::Player(p), n, false);
        }
    }
    Some(())
}
