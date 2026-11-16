//! God-Eternal Rhonas: "Deathtouch. When God-Eternal Rhonas enters, double the power of each other creature you
//! control until end of turn. Those creatures gain vigilance until end of turn. When God-Eternal Rhonas dies or is
//! put into exile from the battlefield, you may put it into its owner's library third from the top."

use super::*;
use crate::card::Duration;

pub const HANDLERS: &[Handler] = &[Handler::effect("GodEternalRhonas.GodEternalRhonasEffect", apply)];

/// each creature you control other than Rhonas, in the battlefield's order: +X/+0 (X its power then) and vigilance
/// until end of turn, each a `FixedTarget` effect
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let creatures: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| *o != cx.source_id() && game.obj(*o).controller == you && game.has_type(*o, CardType::Creature))
        .collect();
    for o in creatures {
        if game.obj(o).zone != Zone::Battlefield {
            continue;
        }
        let power = game.power_toughness(o).0;
        let effects = [
            EffectDef::BoostTargetEffect {
                power: Amount::Fixed(power),
                toughness: Amount::Fixed(0),
                duration: Duration::EndOfTurn,
            },
            EffectDef::GainAbilityTargetEffect {
                ability: Box::new(AbilityDef::VigilanceAbility),
                duration: Duration::EndOfTurn,
            },
        ];
        cx.run_on(game, d, &effects, vec![Target::Obj(o)]);
    }
    Some(())
}
