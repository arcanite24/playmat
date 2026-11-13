//! Skyclave Relic: "When Skyclave Relic enters the battlefield, if it was kicked, create two tapped tokens that are
//! copies of Skyclave Relic."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SkyclaveRelic.CreateTokenCopySourceEffect", apply)];

/// `CreateTokenCopySourceEffect(2, true)`: two token copies of the Relic (as it last was), each entering tapped
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let def = Arc::new((*game.obj(cx.source_id()).copiable()).clone());
    for _ in 0..2 * token_copies(game, you, &def) {
        let token_def = made_token(game, you, def.clone());
        let id = game.create_obj(token_def, you, Zone::Library);
        let o = game.obj_mut(id);
        o.token = true;
        o.trigger_order = (1 << 32) + id.0 as u64;
        enter_battlefield(game, d, id, you, false);
        game.obj_mut(id).tapped = true;
        if d.stopped() {
            return None;
        }
    }
    Some(())
}
