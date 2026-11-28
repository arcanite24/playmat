//! Volatile Rig: "Whenever this creature is dealt damage, flip a coin. If you lose the flip, sacrifice this
//! creature. When this creature dies, flip a coin. If you lose the flip, it deals 4 damage to each creature and each
//! player."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("VolatileRig.VolatileRigEffect", sacrifice_on_loss),
    Handler::effect("VolatileRig.VolatileRigEffect2", damage_on_loss).filters(&["StaticFilters.FILTER_PERMANENT_CREATURE"]),
];

/// `flipCoin(winnable)`; a lost flip sacrifices it
fn sacrifice_on_loss(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let lost = vec![EffectDef::SacrificeSourceEffect];
    cx.run(game, d, &[EffectDef::FlipCoinEffect { won: Vec::new(), lost }]);
    (!d.stopped()).then_some(())
}

/// a lost flip: 4 damage to each creature, then to each player from you
fn damage_on_loss(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let four = crate::card::Amount::Fixed(4);
    let lost = vec![
        EffectDef::DamageAllEffect { amount: four.clone(), filter: cx.filters[0].clone() },
        EffectDef::DamagePlayersEffect { amount: four, controller: "any".into() },
    ];
    cx.run(game, d, &[EffectDef::FlipCoinEffect { won: Vec::new(), lost }]);
    (!d.stopped()).then_some(())
}
