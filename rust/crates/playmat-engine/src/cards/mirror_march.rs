//! Mirror March: "Whenever a nontoken creature you control enters, flip a coin until you lose a flip. For each flip
//! you won, create a token that's a copy of that creature. Those tokens gain haste. Exile them at the beginning of the
//! next end step."

use super::*;
use crate::card::AbilityDef;

pub const HANDLERS: &[Handler] = &[Handler::effect("MirrorMarch.MirrorMarchEffect", apply)];

/// `player.flipCoin(source, game, true)` until a flip is lost ("Heads or tails?", then `RandomUtil.nextBoolean()`),
/// then `CreateTokenCopyTargetEffect(you, null, true, won)` and `exileTokensCreatedAtNextEndStep`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut won = 0;
    loop {
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
            break;
        }
        won += 1;
    }
    let Some(of) = cx.obj(0) else { return Some(()) };
    if won == 0 {
        return Some(());
    }
    let made = token_copy_modified(game, d, of, you, won, &|def| {
        if !def.abilities.contains(&AbilityDef::HasteAbility) {
            def.abilities.push(AbilityDef::HasteAbility);
        }
    });
    if d.stopped() {
        return None;
    }
    remove_tokens_at_next_end_step(game, &made, cx.source_id(), you, true);
    Some(())
}
