//! Invoke Justice: "Return target permanent card from your graveyard to the battlefield, then distribute four +1/+1
//! counters among any number of creatures and/or Vehicles target player controls."

use super::*;
use crate::card::{Amount, Pred, TargetDef};

pub const HANDLERS: &[Handler] = &[Handler::effect("InvokeJustice.InvokeJusticeEffect", apply)];

/// The second target, the player, chooses (`TargetPermanentAmount(4, 0, …)`, not targeted) among the creatures and
/// Vehicles they control, then divides the four counters; nothing when they control none
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(player) = cx.player(1) else { return Some(()) };
    let filter = Filter(vec![
        Pred::Or { xs: vec![Pred::Type { t: CardType::Creature }, Pred::Subtype { s: "Vehicle".into() }] },
        Pred::Controller { who: "you".into() },
    ]);
    let any = game.battlefield.iter().any(|o| game.filter_matches(&filter, player, Some(cx.source_id()), *o));
    if !any {
        return Some(());
    }
    let def = TargetDef::TargetPermanent {
        filter,
        min: 0,
        max: 4,
        distribute: Some(Amount::Fixed(4)),
        x_count: false,
        count: None,
    };
    let rule = item_rule(game, cx.item);
    let Some(chosen) = choose_target(game, d, player, &def, Some(cx.source_id()), &rule, false) else {
        return if d.stopped() { None } else { Some(()) };
    };
    let objs: Vec<ObjId> = chosen.iter().filter_map(|t| if let Target::Obj(o) = t { Some(*o) } else { None }).collect();
    let amounts = distribute(game, d, player, &Amount::Fixed(4), objs.len(), Some(cx.source_id()))?;
    for (o, n) in objs.into_iter().zip(amounts) {
        if game.obj(o).zone == Zone::Battlefield && n > 0 {
            put_counters(game, o, "+1/+1", n, you);
        }
    }
    Some(())
}
