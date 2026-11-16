//! Hero of Leina Tower: "Heroic — Whenever you cast a spell that targets Hero of Leina Tower, you may pay {X}. If you
//! do, put X +1/+1 counters on Hero of Leina Tower."

use super::*;
use crate::card::{CounterDef, ManaCost};

pub const HANDLERS: &[Handler] = &[Handler::effect("HeroOfLeinaTower.HeroOfLeinaTowerEffect", apply)];

/// "Do you want to to pay {X}?", then `announceX(0, MAX)` and {X} paid; paid: X +1/+1 counters on it, while it's
/// still on the battlefield (`AddCountersSourceEffect`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    if !choose_use(game, d, you, "Do you want to to pay {X}?")? {
        return Some(());
    }
    let x = loop {
        match ask(game, d, you, &Query::Amount { min: 0, max: i32::MAX }) {
            Answer::Int(n) if n >= 0 => break n,
            Answer::Stop => return None,
            _ => {}
        }
        if d.stopped() {
            return None;
        }
    };
    if x > 0 {
        let cost = ManaCost { generic: x as u32, ..ManaCost::default() };
        let before = game.clone();
        if !pay_mana(game, d, you, &cost) {
            if d.stopped() {
                return None;
            }
            *game = before;
            return Some(());
        }
    }
    let counter = CounterDef { name: "+1/+1".into(), amount: x };
    cx.run(game, d, &[EffectDef::AddCountersSourceEffect { counter, amount: None }]);
    Some(())
}
