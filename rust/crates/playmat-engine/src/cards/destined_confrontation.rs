//! Destined Confrontation: "Each player chooses any number of creatures they control with total power 4 or less,
//! then sacrifices all other creatures they control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DestinedConfrontation.DestinedConfrontationEffect", apply)];

/// Each player in range in turn from you chooses (`choose(DestinedConfrontationTarget)`, not targeted: its
/// `possibleTargets` are every creature they control not chosen yet, its `canTarget` power check isn't asked of
/// those) until they're done or none is left; then every creature on the battlefield not chosen is sacrificed, in
/// battlefield order
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let n = game.seats();
    let mut kept: Vec<ObjId> = Vec::new();
    for p in (0..n).map(|i| (you + i) % n) {
        if game.player(p).lost {
            continue;
        }
        let mut chosen: Vec<ObjId> = Vec::new();
        loop {
            let can: Vec<ObjId> = game
                .battlefield
                .iter()
                .copied()
                .filter(|o| {
                    game.obj(*o).controller == p && game.has_type(*o, CardType::Creature) && !chosen.contains(o)
                })
                .collect();
            match ask(game, d, p, &Query::Cards { objs: can.clone(), required: false }) {
                Answer::Obj(o) if chosen.contains(&o) => chosen.retain(|c| *c != o),
                Answer::Obj(o) if can.contains(&o) => {
                    chosen.push(o);
                    if can.len() == 1 {
                        break;
                    }
                }
                Answer::Stop => return None,
                Answer::Obj(_) | Answer::Player(_) | Answer::Unknown => {}
                _ => break,
            }
            if d.stopped() {
                return None;
            }
        }
        kept.extend(chosen);
    }
    let doomed: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.has_type(*o, CardType::Creature) && !kept.contains(o))
        .collect();
    for o in doomed {
        sacrifice(game, o);
    }
    Some(())
}
