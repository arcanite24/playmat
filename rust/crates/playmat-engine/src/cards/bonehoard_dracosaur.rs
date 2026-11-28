//! Bonehoard Dracosaur: "At the beginning of your upkeep, exile the top two cards of your library. You may play
//! them this turn. If you exiled a land card this way, create a 3/1 red Dinosaur creature token. If you exiled a
//! nonland card this way, create a Treasure token."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BonehoardDracosaur.BoneahoardDracosaurEffect", apply).effects(&[
    "new CreateTokenEffect(new Dinosaur31Token())",
    "new CreateTokenEffect(new TreasureToken())",
])];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let cards: Vec<ObjId> = game.player(you).library.iter().take(2).copied().collect();
    if cards.is_empty() {
        return Some(());
    }
    // into an exile zone of their own (not the source's), playable this turn (`PlayFromNotOwnHandZoneTargetEffect`)
    for c in &cards {
        game.move_to(*c, Zone::Exile);
    }
    let cards: Vec<ObjId> = cards.into_iter().filter(|c| game.obj(*c).zone == Zone::Exile).collect();
    let turn = game.turn;
    for c in &cards {
        game.obj_mut(*c).may_play = Some((you, turn));
    }
    let land = cards.iter().any(|c| game.obj(*c).is(CardType::Land));
    let nonland = cards.iter().any(|c| !game.obj(*c).is(CardType::Land));
    if land {
        cx.run(game, d, &cx.effects[..1]);
        if d.stopped() {
            return None;
        }
    }
    if nonland {
        cx.run(game, d, &cx.effects[1..2]);
    }
    Some(())
}
