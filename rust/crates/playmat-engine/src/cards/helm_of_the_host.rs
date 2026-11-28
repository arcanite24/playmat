//! Helm of the Host: "At the beginning of combat on your turn, create a token that's a copy of equipped creature,
//! except the token isn't legendary. That token gains haste."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("HelmOfTheHost.HelmOfTheHostEffect", apply)];

/// The creature the Equipment is attached to (as it last was on the battlefield, if it has left), copied by
/// `CreateTokenCopyTargetEffect(you, null, true)` with `setIsntLegendary(true)`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let helm = game.obj(cx.source_id());
    let attached = if helm.zone == Zone::Battlefield { helm.attached_to } else { helm.lki_attached_to };
    let Some((creature, _)) = attached else { return Some(()) };
    let copy: EffectDef = serde_json::from_value(serde_json::json!({
        "class": "CreateTokenCopyTargetEffect", "not_legendary": true, "haste": true
    }))
    .expect("a token copy effect");
    cx.run_on(game, d, &[copy], vec![Target::Obj(creature)]);
    if d.stopped() {
        return None;
    }
    Some(())
}
