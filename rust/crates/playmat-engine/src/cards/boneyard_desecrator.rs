//! Boneyard Desecrator: "Menace. {1}{B}, Sacrifice another creature: Put a +1/+1 counter on Boneyard Desecrator. If
//! an outlaw was sacrificed this way, create a Treasure token. (Assassins, Mercenaries, Pirates, Rogues, and Warlocks
//! are outlaws.)"

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("BoneyardDesecrator.BoneyardDesecratorEffect", apply).tokens(&["TreasureToken"])];

const OUTLAWS: [&str; 5] = ["Assassin", "Mercenary", "Pirate", "Rogue", "Warlock"];

/// a Treasure (one) if a permanent the cost sacrificed was an outlaw as it last was on the battlefield
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let outlaw = cx.item.sacrificed.iter().any(|p| {
        match game.watch.left_battlefield.iter().rev().find(|l| l.id == *p) {
            Some(l) => {
                l.subtypes.iter().any(|s| OUTLAWS.contains(&s.as_str())) || game.obj(*p).def.all_creature_types
            }
            None => OUTLAWS.iter().any(|s| game.has_subtype(*p, s)),
        }
    });
    if outlaw {
        cx.create_tokens(game, d, &cx.tokens[0], 1);
    }
    Some(())
}
