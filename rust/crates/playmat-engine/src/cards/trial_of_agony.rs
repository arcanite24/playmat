//! Trial of Agony: "Choose two target creatures controlled by the same opponent. That player chooses one of those
//! creatures. Trial of Agony deals 5 damage to that creature, and the other can't block this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TrialOfAgony.TrialOfAgonyEffect", apply)
    .effects(&["new CantBlockTargetEffect(Duration.EndOfTurn)"])];

/// The targets still on the battlefield: one alone takes the damage; with two, their controller chooses which one
/// (`player.choose` of a not-target `TargetPermanent`, asked even with one left), and the other can't block this turn.
/// Java's `RandomUtil.randomFromCollection` of the one left draws nothing.
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let mut left: Vec<ObjId> = cx.objs(0).into_iter().filter(|o| game.obj(*o).zone == Zone::Battlefield).collect();
    let (damaged, cant_block) = match left.len() {
        0 => return Some(()),
        1 => (Some(left[0]), None),
        _ => {
            let chooser = game.obj(left[0]).controller;
            let picked = loop {
                match ask(game, d, chooser, &Query::Cards { objs: left.clone(), required: true }) {
                    Answer::Obj(o) if left.contains(&o) => break Some(o),
                    Answer::Stop => return None,
                    _ => {}
                }
                if d.stopped() {
                    return None;
                }
            };
            if let Some(p) = picked {
                left.retain(|o| *o != p);
            }
            (picked, left.first().copied())
        }
    };
    if let Some(o) = damaged {
        deal_damage(game, cx.source_id(), Target::Obj(o), 5, false);
    }
    if let Some(o) = cant_block {
        cx.run_on(game, d, &cx.effects[..1], vec![Target::Obj(o)]);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}
