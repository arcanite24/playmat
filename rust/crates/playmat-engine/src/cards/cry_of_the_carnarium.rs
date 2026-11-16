//! Cry of the Carnarium: "All creatures get -2/-2 until end of turn. Exile all creature cards in all graveyards that
//! were put there from the battlefield this turn. If a creature would die this turn, exile it instead."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("CryOfTheCarnarium.CryOfTheCarnariumExileEffect", exile),
    Handler::replacement(
        "CryOfTheCarnarium.CryOfTheCarnariumReplacementEffect",
        &[EventType::ZoneChange],
        dies,
        exile_instead,
    ),
];

/// `CardsPutIntoGraveyardWatcher.getCardsPutIntoGraveyardFromBattlefield`: the cards put into a graveyard from the
/// battlefield this turn that are still there (the same zone change count); the creature cards among them exiled
fn exile(game: &mut Game, _: &mut dyn Decider, _: &EffectCx) -> Option<()> {
    let mut cards: Vec<ObjId> = Vec::new();
    for (c, zcc, from_battlefield) in game.watch.to_graveyard.clone() {
        let o = game.obj(c);
        if from_battlefield
            && o.zcc == zcc
            && o.zone == Zone::Graveyard
            && game.has_type(c, CardType::Creature)
            && !cards.contains(&c)
        {
            cards.push(c);
        }
    }
    for c in cards {
        game.move_to(c, Zone::Exile);
    }
    Some(())
}

/// `isDiesEvent` of a creature (resolved: until the turn's cleanup, `Duration.EndOfTurn`)
fn dies(game: &Game, _cx: &StaticCx, ev: &Event) -> bool {
    let Some(id) = ev.target else { return false };
    ev.from == Some(Zone::Battlefield) && ev.to == Some(Zone::Graveyard) && game.has_type(id, CardType::Creature)
}

/// exiled instead
fn exile_instead(_game: &mut Game, _cx: &StaticCx, ev: &mut Event) -> bool {
    ev.to = Some(Zone::Exile);
    false
}
