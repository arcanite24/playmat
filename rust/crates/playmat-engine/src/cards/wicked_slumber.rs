//! Wicked Slumber: "Tap up to two target creatures. Put a stun counter on either of them. Then put a stun counter on
//! either of them."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("WickedSlumber.WickedSlumberEffect", apply)];

/// one target left: both counters on it; two: each counter on the one you choose (not targeted)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let them: Vec<ObjId> = cx.objs(0).into_iter().filter(|o| game.obj(*o).zone == Zone::Battlefield).collect();
    match them.len() {
        0 => {}
        1 => {
            put_counters(game, them[0], "stun", 1, you);
            put_counters(game, them[0], "stun", 1, you);
        }
        _ => {
            for _ in 0..2 {
                let chosen = loop {
                    match ask(game, d, you, &Query::Cards { objs: them.clone(), required: true }) {
                        Answer::Obj(o) if them.contains(&o) => break o,
                        Answer::Stop => return None,
                        _ => {}
                    }
                    if d.stopped() {
                        return None;
                    }
                };
                put_counters(game, chosen, "stun", 1, you);
            }
        }
    }
    Some(())
}
