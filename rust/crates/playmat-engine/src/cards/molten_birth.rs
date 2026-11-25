//! Molten Birth: "Create two 1/1 red Elemental creature tokens. Then flip a coin. If you win the flip, return Molten
//! Birth to its owner's hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MoltenBirth.MoltenBirthEffect", apply)];

/// `controller.flipCoin(source, game, true)` as `FlipCoinEffect` asks and logs it; won, the spell (still on the stack
/// as it resolves) `moveCards` to its owner's hand
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let chosen = choose_use(game, d, you, "Heads or tails?")?;
    let result = game.rng.next_boolean();
    if log::kept(game) {
        let name = |b: bool| if b { "Heads" } else { "Tails" };
        let from = log::source(game).map(|s| format!(" (source: {})", log::object(game, s))).unwrap_or_default();
        let p = log::player(you);
        log::inform(game, format!("{} chose {}", p, name(chosen)));
        log::inform(game, format!("{} flipped {}{}", p, name(result), from));
        let outcome = if result == chosen { "won" } else { "lost" };
        log::inform(game, format!("{} {} the flip{}", p, outcome, from));
    }
    let (me, zcc) = cx.source;
    if result == chosen && game.obj(me).zone == Zone::Stack && game.obj(me).zcc == zcc {
        game.move_to(me, Zone::Hand);
    }
    Some(())
}
