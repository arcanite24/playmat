//! The Fire Crystal: "{4}{R}{R}, {T}: Create a token that's a copy of target creature you control. Sacrifice it at
//! the beginning of the next end step."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TheFireCrystal.TheFireCrystalEffect", apply)];

/// `CreateTokenCopyTargetEffect` of the target, then `sacrificeTokensCreatedAtNextEndStep` (as Reflection of
/// Kiki-Jiki's copies)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(of) = cx.obj(0) else { return Some(()) };
    if game.obj(of).zone != Zone::Battlefield {
        return Some(());
    }
    let made = token_copy_with(game, d, of, cx.controller(), None, false);
    let delayed = Delayed {
        rule: "At the beginning of the next end step, sacrifice the token copies.".into(),
        sacrifice: made.iter().map(|t| (*t, game.obj(*t).zcc)).collect(),
        warp_exile: Vec::new(),
        effects: Vec::new(),
        source_zcc: 0,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    game.at_next_end_step.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), cx.controller()) });
    Some(())
}
