//! Memorial Vault: "{T}, Sacrifice another artifact: Exile the top X cards of your library, where X is one plus the
//! mana value of the sacrificed artifact. You may play those cards this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MemorialVault.MemorialVaultEffect", apply)];

/// one plus the sacrificed artifact's mana value (`SacrificeTargetCost.getPermanents`), then
/// `ExileTopXMayPlayUntilEffect(x, Duration.EndOfTurn)`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let x = 1 + cx.item.sacrificed_stats[2];
    let e = EffectDef::ExileTopXMayPlayUntilEffect {
        amount: Amount::Fixed(x),
        choose_one: false,
        duration: "EndOfTurn".into(),
    };
    cx.run(game, d, &[e]);
    (!d.stopped()).then_some(())
}
