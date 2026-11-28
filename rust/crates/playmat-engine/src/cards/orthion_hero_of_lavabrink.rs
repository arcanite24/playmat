//! Orthion, Hero of Lavabrink: "{1}{R}, {T}: Create a token that's a copy of another target creature you control. It
//! gains haste. Sacrifice it at the beginning of the next end step. Activate only as a sorcery. {6}{R}{R}{R}, {T}:
//! Create five tokens that are copies of another target creature you control. They gain haste. Sacrifice them at the
//! beginning of the next end step. Activate only as a sorcery."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("OrthionHeroOfLavabrink.OrthionHeroOfLavabrinkEffect", apply)];

/// the argument says five (`true`) or one
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(of) = cx.obj(0) else { return Some(()) };
    if game.obj(of).zone != Zone::Battlefield {
        return Some(());
    }
    let n = if cx.args.first().map(|a| a.trim()) == Some("true") { 5 } else { 1 };
    let made = token_copy_modified(game, d, of, you, n, &|_: &mut CardDef| {});
    if d.stopped() {
        return None;
    }
    for t in &made {
        if game.obj(*t).zone == Zone::Battlefield {
            grant(game, *t, &AbilityDef::HasteAbility);
        }
    }
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
    game.at_next_end_step.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), you) });
    Some(())
}
