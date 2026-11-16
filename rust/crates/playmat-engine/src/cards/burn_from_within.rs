//! Burn from Within: "Burn from Within deals X damage to any target. If a creature is dealt damage this way, it loses
//! indestructible until end of turn. If that creature would die this turn, exile it instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BurnFromWithin.BurnFromWithinEffect", apply)];

/// A creature: the "exile it instead" replacement first, then the X damage, and indestructible lost if any was
/// dealt; a player: the damage
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let x = cx.item.x;
    match cx.target(0)? {
        Target::Obj(o) => {
            let zcc = game.obj(o).zcc;
            game.exile_if_dies.push((o, zcc));
            if deal_damage(game, cx.source_id(), Target::Obj(o), x, false) > 0 {
                let lose = EffectDef::LoseAbilityTargetEffect {
                    ability: Box::new(AbilityDef::IndestructibleAbility),
                    until_your_next_turn: false,
                };
                cx.run_on(game, d, &[lose], vec![Target::Obj(o)]);
            }
        }
        Target::Player(p) => {
            deal_damage(game, cx.source_id(), Target::Player(p), x, false);
        }
    }
    Some(())
}
