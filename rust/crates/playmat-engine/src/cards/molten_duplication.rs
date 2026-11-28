//! Molten Duplication: "Create a token that's a copy of target artifact or creature you control, except it's an
//! artifact in addition to its other types. It gains haste until end of turn. Sacrifice it at the beginning of the
//! next end step."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MoltenDuplication.MoltenDuplicationEffect", apply)];

/// `CreateTokenCopyTargetEffect(null, ARTIFACT, false)`, then haste for the tokens made until end of turn, then
/// `sacrificeTokensCreatedAtNextEndStep` ("sacrifice the token copies")
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(of) = cx.obj(0) else { return Some(()) };
    let made = token_copy_modified(game, d, of, you, 1, &|def: &mut CardDef| {
        if !def.types.contains(&CardType::Artifact) {
            def.types.push(CardType::Artifact);
        }
    });
    if d.stopped() {
        return None;
    }
    let made: Vec<ObjId> = made.into_iter().filter(|t| game.obj(*t).zone == Zone::Battlefield).collect();
    for t in &made {
        grant(game, *t, &AbilityDef::HasteAbility);
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
