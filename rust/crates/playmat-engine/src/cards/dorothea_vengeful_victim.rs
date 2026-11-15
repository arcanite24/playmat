//! Dorothea, Vengeful Victim // Dorothea's Retribution: "Enchanted creature has \"Whenever this creature attacks,
//! create a 4/4 white Spirit creature token with flying that's tapped and attacking. Sacrifice that token at end of
//! combat.\""

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DorotheaVengefulVictim.DorotheasRetributionEffect", apply)
    .tokens(&["DorotheasRetributionSpiritToken"])];

/// the token, tapped and attacking, then an "at end of combat" delayed trigger sacrificing it
/// (`SacrificeTargetEffect` with `FixedTargets`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let e = EffectDef::CreateTokenEffect {
        token: cx.tokens[0].clone(),
        amount: Amount::Fixed(1),
        tapped_and_attacking: true,
        tapped: false,
        attacking: false,
        sacrifice_at_end_of_combat: true,
        sacrifice_at_next_end_step: false,
        haste_until_end_of_turn: false,
        also: None,
        attach_source: None,
    };
    cx.run_on(game, d, &[e], Vec::new());
    Some(())
}
