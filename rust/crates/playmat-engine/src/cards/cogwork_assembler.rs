//! Cogwork Assembler: "{7}: Create a token that's a copy of target artifact. That token gains haste. Exile it at the
//! beginning of the next end step."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("CogworkAssembler.CogworkAssemblerCreateTokenEffect", apply)];

/// `CreateTokenCopyTargetEffect(null, ARTIFACT, true)` of the target still on the battlefield, then an "at the
/// beginning of the next end step, exile" for each token made
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    copy_with_haste_exiled(game, d, cx, CardType::Artifact)
}

/// (Felhide Spiritbinder's too, an enchantment)
pub(super) fn copy_with_haste_exiled(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx, add: CardType) -> Option<()> {
    let you = cx.controller();
    let Some(of) = cx.obj(0) else { return Some(()) };
    let made = token_copy_modified(game, d, of, you, 1, &|def: &mut CardDef| {
        if !def.types.contains(&add) {
            def.types.push(add);
        }
        if !def.abilities.contains(&AbilityDef::HasteAbility) {
            def.abilities.push(AbilityDef::HasteAbility);
        }
    });
    if d.stopped() {
        return None;
    }
    for t in made {
        let delayed = Delayed {
            rule: "At the beginning of the next end step, exile that permanent.".into(),
            sacrifice: Vec::new(),
            warp_exile: Vec::new(),
            effects: vec![EffectDef::ExileTargetEffect],
            source_zcc: cx.source.1,
            fixed: vec![(t, game.obj(t).zcc)],
            return_tapped: Vec::new(),
            targets: Vec::new(),
            condition: None,
        };
        game.at_next_end_step
            .push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), you) });
    }
    Some(())
}
