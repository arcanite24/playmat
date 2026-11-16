//! Feast of the Victorious Dead: "At the beginning of your end step, if one or more creatures died this turn, you
//! gain that much life and distribute that many +1/+1 counters among any number of creatures you control."

use super::*;
use crate::card::{Amount, Pred, TargetDef};

pub const HANDLERS: &[Handler] = &[
    Handler::condition("FeastOfTheVictoriousDead.FeastOfTheVictoriousDeadCondition", any_died),
    Handler::effect("FeastOfTheVictoriousDead.FeastOfTheVictoriousDeadEffect", apply),
];

/// `CreaturesDiedThisTurnCount` above 0
fn any_died(game: &Game, _: Seat, _: ObjId) -> bool {
    game.creatures_died > 0
}

/// That many +1/+1 counters divided among 1 to that many creatures you control (`TargetCreaturePermanentAmount`,
/// not targeted: `chooseTargetAmount`, the creatures, then the division)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let amount = game.creatures_died as i32;
    if amount <= 0 || controlled_creatures(game, you).is_empty() {
        return Some(());
    }
    let def = TargetDef::TargetPermanent {
        filter: Filter(vec![Pred::Type { t: CardType::Creature }, Pred::Controller { who: "you".into() }]),
        min: 1,
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
