//! Two-Headed Giant: "Whenever Two-Headed Giant attacks, flip two coins. If both coins come up heads, Two-Headed
//! Giant gains double strike until end of turn. If both coins come up tails, Two-Headed Giant gains menace until end
//! of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TwoHeadedGiant.TwoHeadedGiantEffect", apply).effects(&[
    "new GainAbilitySourceEffect(DoubleStrikeAbility.getInstance(), Duration.EndOfTurn)",
    "new GainAbilitySourceEffect(new MenaceAbility(), Duration.EndOfTurn)",
])];

/// `flipCoins(2, winnable false)`: no call, each flip a random boolean
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut flips = [false; 2];
    for f in &mut flips {
        *f = game.rng.next_boolean();
        if log::kept(game) {
            let from = log::source(game).map(|s| format!(" (source: {})", log::object(game, s))).unwrap_or_default();
            let name = if *f { "Heads" } else { "Tails" };
            log::inform(game, format!("{} flipped {}{}", log::player(you), name, from));
        }
    }
    if flips[0] == flips[1] {
        let i = if flips[0] { 0 } else { 1 };
        cx.run(game, d, &cx.effects[i..i + 1]);
    }
    (!d.stopped()).then_some(())
}
