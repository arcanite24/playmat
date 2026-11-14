//! Goblin Kaboomist: "At the beginning of your upkeep, create a colorless artifact token named Land Mine with
//! "{R}, Sacrifice this token: It deals 2 damage to target attacking creature without flying." Then flip a coin. If
//! you lose the flip, Goblin Kaboomist deals 2 damage to itself."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("GoblinKaboomist.GoblinKaboomistFlipCoinEffect", apply)];

/// with the Goblin still on the battlefield: `PlayerImpl.flipCoin(source, game, true)` (as `FlipCoinEffect`), and on
/// a lost flip it deals 2 damage to itself
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let me = cx.source_id();
    if game.obj(me).zone != Zone::Battlefield {
        return Some(());
    }
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
    if result != chosen {
        if log::kept(game) {
            let line = format!("{} deals 2 damage to itself", log::object(game, me));
            log::inform(game, line);
        }
        deal_damage(game, me, Target::Obj(me), 2, false);
    }
    Some(())
}
