//! Labyrinth Raptor: "Whenever a creature you control with menace becomes blocked, defending player sacrifices a
//! creature blocking it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("LabyrinthRaptor.LabyrinthRaptorEffect", apply)];

/// the blocked creature is the trigger's target pointer; while it's still attacking, its defending player (in a duel, its
/// controller's opponent) sacrifices one of the creatures blocking it (`SacrificeEffect` over
/// `BlockingAttackerIdPredicate`), chosen when there's more than one
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let attacker = cx.obj(0)?;
    // `Combat.getDefendingPlayerId`: none once it left combat (it died, or was removed)
    if game.obj(attacker).zone != Zone::Battlefield || game.obj(attacker).attacking.is_none() {
        return Some(());
    }
    let defender = game.opponent(game.obj(attacker).controller);
    let possible: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|b| {
            let o = game.obj(*b);
            o.controller == defender && o.is(CardType::Creature) && o.blocking.contains(&attacker)
        })
        .collect();
    if possible.is_empty() {
        return Some(());
    }
    let chosen = choose_cards(game, d, defender, &possible, &possible, 1, 1, "sacrifice")?;
    for o in chosen {
        if game.obj(o).zone == Zone::Battlefield {
            sacrifice(game, o);
        }
    }
    Some(())
}
