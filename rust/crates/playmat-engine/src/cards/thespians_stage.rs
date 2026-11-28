//! Thespian's Stage: "{2}, {T}: Thespian's Stage becomes a copy of target land, except it has this ability."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ThespiansStage.ThespiansStageCopyEffect", apply)];

/// `game.copyPermanent` with no end: the target's copiable values, then this ability added after its own (the new
/// copiable values, 707.9b), until the Stage leaves the battlefield
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(of) = cx.obj(0) else { return Some(()) };
    if !cx.source_on_battlefield(game) || game.obj(of).zone != Zone::Battlefield {
        return Some(());
    }
    let src = cx.source_id();
    let mine = |a: &AbilityDef| match a {
        AbilityDef::SimpleActivatedAbility { effects, .. } => effects.iter().any(|e| {
            matches!(e, EffectDef::CardEffect { handler, .. } if handler.java() == "ThespiansStage.ThespiansStageCopyEffect")
        }),
        _ => false,
    };
    let Some(ability) = game.obj(src).def.abilities.iter().find(|a| mine(a)).cloned() else { return Some(()) };
    let mut copy = (*game.obj(of).copiable()).clone();
    copy.abilities.push(ability);
    let copy = Arc::new(copy);
    let o = game.obj_mut(src);
    if o.printed_def.is_none() {
        o.printed_def = Some(o.def.clone());
    }
    o.copied_def = Some(copy.clone());
    o.def = copy;
    Some(())
}
