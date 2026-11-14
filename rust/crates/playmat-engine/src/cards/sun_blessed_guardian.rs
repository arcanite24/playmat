//! Sun-Blessed Guardian // Furnace-Blessed Conqueror: "Whenever Furnace-Blessed Conqueror attacks, create a tapped
//! and attacking token that's a copy of it. Put a +1/+1 counter on that token for each +1/+1 counter on
//! Furnace-Blessed Conqueror. Sacrifice the token at the beginning of the next end step."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SunBlessedGuardian.SunBlessedGuardianEffect", apply)
    .effects(&["new CreateTokenCopyTargetEffect(null, null, false, 1, true, true)"])];

/// The source still on the battlefield: the tapped and attacking token copy of it, its sacrifice at the next end
/// step (`sacrificeTokensCreatedAtNextEndStep`), then the source's +1/+1 counters on each token
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let src = cx.source_id();
    if !cx.source_on_battlefield(game) {
        return Some(());
    }
    let first = game.objs.len();
    cx.run_on(game, d, &cx.effects[..1], vec![Target::Obj(src)]);
    if d.stopped() {
        return None;
    }
    let made: Vec<ObjId> = (first..game.objs.len())
        .map(|i| ObjId(i as u32))
        .filter(|o| game.obj(*o).token && game.obj(*o).zone == Zone::Battlefield)
        .collect();
    remove_tokens_at_next_end_step(game, &made, src, you, false);
    let n = game.obj(src).counter("+1/+1");
    if n > 0 {
        for t in made {
            put_counters(game, t, "+1/+1", n, you);
        }
    }
    Some(())
}
