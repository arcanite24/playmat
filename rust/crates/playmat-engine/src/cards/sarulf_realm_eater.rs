//! Sarulf, Realm Eater: "Whenever a permanent an opponent controls is put into a graveyard from the battlefield, put a
//! +1/+1 counter on Sarulf. At the beginning of your upkeep, if Sarulf has one or more +1/+1 counters on it, you may
//! remove all of them. If you do, exile each other nonland permanent with mana value less than or equal to the
//! number of counters removed this way."

use super::*;
use crate::card::{Cmp, Filter, Pred};

pub const HANDLERS: &[Handler] = &[Handler::effect("SarulfRealmEater.SarulfRealmEaterEffect", apply)];

/// Sarulf still there: its +1/+1 counters removed, then every other nonland permanent with mana value at most that
/// many exiled together
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if !cx.source_on_battlefield(game) {
        return Some(());
    }
    let src = cx.source_id();
    let n = game.obj(src).counter("+1/+1").max(0);
    if n > 0 {
        let o = game.obj_mut(src);
        o.add_counters("+1/+1", -n);
        o.counters.retain(|(_, k)| *k > 0);
    }
    let filter = Filter::of(vec![
        Pred::Not { x: Box::new(Pred::Type { t: CardType::Land }) },
        Pred::Mv { cmp: Cmp::OrLess, n },
        Pred::Another,
    ]);
    cx.run(game, d, &[EffectDef::ExileAllEffect { filter }]);
    if d.stopped() {
        return None;
    }
    Some(())
}
