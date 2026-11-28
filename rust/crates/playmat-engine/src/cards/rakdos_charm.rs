//! Rakdos Charm: "Choose one — Exile all cards from target player's graveyard; or destroy target artifact; or each
//! creature deals 1 damage to its controller."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RakdosCharm.RakdosCharmDamageEffect", apply)];

/// Each creature on the battlefield, in battlefield order, deals 1 damage to its controller
/// (`Player.damage(1, permanent, …)`: noncombat damage from the creature)
fn apply(game: &mut Game, _d: &mut dyn Decider, _cx: &EffectCx) -> Option<()> {
    let creatures: Vec<ObjId> =
        game.battlefield.iter().copied().filter(|o| game.has_type(*o, CardType::Creature)).collect();
    for o in creatures {
        if game.obj(o).zone != Zone::Battlefield {
            continue;
        }
        let p = game.obj(o).controller;
        deal_damage(game, o, Target::Player(p), 1, false);
    }
    Some(())
}
