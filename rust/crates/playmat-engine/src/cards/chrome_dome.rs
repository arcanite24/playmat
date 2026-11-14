//! Chrome Dome: "{5}: Create a token that's a copy of another target artifact you control. That token gains haste.
//! Sacrifice it at the beginning of the next end step."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ChromeDome.ChromeDomeEffect", apply)];

/// `CreateTokenCopyTargetEffect().setHasHaste(true)` of the target still on the battlefield, then
/// `sacrificeTokensCreatedAtNextEndStep`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(of) = cx.obj(0) else { return Some(()) };
    if game.obj(of).zone != Zone::Battlefield {
        return Some(());
    }
    let made = token_copy_modified(game, d, of, you, 1, &|def: &mut CardDef| {
        if !def.abilities.contains(&AbilityDef::HasteAbility) {
            def.abilities.push(AbilityDef::HasteAbility);
        }
    });
    if d.stopped() {
        return None;
    }
    let made: Vec<ObjId> = made.into_iter().filter(|t| game.obj(*t).zone == Zone::Battlefield).collect();
    remove_tokens_at_next_end_step(game, &made, cx.source_id(), you, false);
    Some(())
}
