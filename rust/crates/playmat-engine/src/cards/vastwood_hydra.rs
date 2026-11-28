//! Vastwood Hydra: "Vastwood Hydra enters with X +1/+1 counters on it. When Vastwood Hydra dies, you may distribute
//! a number of +1/+1 counters equal to the number of +1/+1 counters on Vastwood Hydra among any number of creatures
//! you control."

use super::*;
use crate::card::{Amount, Pred, TargetDef};

pub const HANDLERS: &[Handler] = &[Handler::effect("VastwoodHydra.VastwoodHydraDistributeEffect", apply)];

/// `CountersSourceCount(P1P1)` (as it last was on the battlefield) divided among 0 to that many creatures you
/// control (`TargetCreaturePermanentAmount(amount, 0, amount, …)`, not targeted: `chooseTargetAmount`, the creatures,
/// then the division)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let src = game.obj(cx.source_id());
    let amount = if src.zone == Zone::Battlefield {
        src.counter("+1/+1")
    } else {
        src.lki_counters.iter().filter(|(c, _)| c == "+1/+1").map(|c| c.1).sum()
    };
    if amount <= 0 || controlled_creatures(game, you).is_empty() {
        return Some(());
    }
    let def = TargetDef::TargetPermanent {
        filter: Filter(vec![Pred::Type { t: CardType::Creature }, Pred::Controller { who: "you".into() }]),
        min: 0,
        max: amount as usize,
        distribute: Some(Amount::Fixed(amount)),
        x_count: false,
        count: None,
    };
    let rule = item_rule(game, cx.item);
    let Some(chosen) = choose_target(game, d, you, &def, Some(cx.source_id()), &rule, false) else {
        return if d.stopped() { None } else { Some(()) };
    };
    let objs: Vec<ObjId> = chosen.iter().filter_map(|t| if let Target::Obj(o) = t { Some(*o) } else { None }).collect();
    let amounts = distribute(game, d, you, &Amount::Fixed(amount), objs.len(), Some(cx.source_id()))?;
    for (o, n) in objs.into_iter().zip(amounts) {
        if game.obj(o).zone == Zone::Battlefield && n > 0 {
            put_counters(game, o, "+1/+1", n, you);
        }
    }
    Some(())
}
