//! Saheeli, the Sun's Brilliance: "{U}{R}, {T}: Create a token that's a copy of another target creature or artifact
//! you control, except it's an artifact in addition to its other types. It gains haste. Sacrifice it at the beginning
//! of the next end step."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("SaheeliTheSunsBrilliance.SaheeliTheSunsBrillianceEffect", apply)];

/// The target still there: `CreateTokenCopyTargetEffect().setBecomesArtifact(true).setHasHaste(true)`, then
/// `sacrificeTokensCreatedAtNextEndStep`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(of) = cx.obj(0) else { return Some(()) };
    if game.obj(of).zone != Zone::Battlefield {
        return Some(());
    }
    let first = game.objs.len();
    let e = EffectDef::CreateTokenCopyTargetEffect {
        number: None,
        becomes_artifact: true,
        not_legendary: false,
        add_types: Vec::new(),
        haste: true,
        flying: false,
        power: None,
        toughness: None,
        only_subtype: None,
        add_subtypes: Vec::new(),
        only_color: Vec::new(),
        abilities: Vec::new(),
        legendary: false,
        tapped: false,
        attacking: false,
    };
    cx.run(game, d, &[e]);
    if d.stopped() {
        return None;
    }
    let made: Vec<ObjId> = (first..game.objs.len())
        .map(|i| ObjId(i as u32))
        .filter(|o| game.obj(*o).token && game.obj(*o).zone == Zone::Battlefield)
        .collect();
    remove_tokens_at_next_end_step(game, &made, cx.source_id(), cx.controller(), false);
    Some(())
}
