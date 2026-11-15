//! Grub's Command: "Choose two — • Create a token that's a copy of target Goblin you control. • Creatures target
//! player controls get +1/+1 and gain haste until end of turn. • Destroy target artifact or creature. • Target player
//! mills five cards, then puts each Goblin card milled this way into their hand."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("GrubsCommand.GrubsCommandBoostEffect", boost_mode),
    Handler::effect("GrubsCommand.GrubsCommandMillEffect", mill_mode),
];

/// The creatures the player controls as it resolves, in battlefield order: +1/+1, then haste, until end of turn
fn boost_mode(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = cx.player(0) else { return Some(()) };
    let creatures: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == p && game.has_type(*o, CardType::Creature))
        .collect();
    for o in &creatures {
        boost(game, *o, 1, 1, crate::card::Duration::EndOfTurn);
    }
    for o in creatures {
        grant(game, o, &AbilityDef::HasteAbility);
    }
    Some(())
}

/// With a card in their library: five milled, then the Goblin cards among those that reached the graveyard go to
/// their hand together
fn mill_mode(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = cx.player(0) else { return Some(()) };
    if game.player(p).library.is_empty() {
        return Some(());
    }
    let top: Vec<ObjId> = game.player(p).library.iter().take(5).copied().collect();
    note(game, EventKind::Milled, Some(p), None, None, top.len() as i32);
    for c in &top {
        game.move_to(*c, Zone::Graveyard);
    }
    let goblins: Vec<ObjId> =
        top.into_iter().filter(|c| game.obj(*c).zone == Zone::Graveyard && game.has_subtype(*c, "Goblin")).collect();
    for c in goblins {
        game.move_to(c, Zone::Hand);
    }
    Some(())
}
