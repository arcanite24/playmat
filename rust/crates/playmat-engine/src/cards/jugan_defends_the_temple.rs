//! Remnant of the Rising Star (Jugan Defends the Temple's back face): "Whenever another creature you control enters,
//! you may pay {X}. When you do, put X +1/+1 counters on that creature."

use super::*;
use crate::card::{CounterDef, ManaCost};

pub const HANDLERS: &[Handler] = &[Handler::effect("JuganDefendsTheTemple.RemnantOfTheRisingStarEffect", apply)];

const NUMBERS: [&str; 21] = [
    "zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven", "twelve",
    "thirteen", "fourteen", "fifteen", "sixteen", "seventeen", "eighteen", "nineteen", "twenty",
];

/// Wildborn Preserver's "Pay {X}?", `announceX(0, MAX)` and the cost paid; paid, the reflexive trigger putting X
/// +1/+1 counters on the creature that entered (`permanentEnteringBattlefield`, a `FixedTarget`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    if !choose_use(game, d, you, "Pay {X}?")? {
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
    let cost = ManaCost { generic: x as u32, ..ManaCost::default() };
    let before = game.clone();
    if !pay_mana(game, d, you, &cost) {
        if d.stopped() {
            return None;
        }
        *game = before;
        return Some(());
    }
    let entered = cx.item.entering?;
    // `CardUtil.getAddRemoveCountersText`: the counter's description ("a +1/+1 counter", "three +1/+1 counters")
    let counters = match x {
        1 => "a +1/+1 counter".to_string(),
        0..=20 => format!("{} +1/+1 counters", NUMBERS[x as usize]),
        _ => format!("{x} +1/+1 counters"),
    };
    let counter = CounterDef { name: "+1/+1".into(), amount: x };
    let delayed = Delayed {
        rule: format!("Put {counters} on that creature."),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![EffectDef::AddCountersTargetEffect { counter, value: None, second: false }],
        source_zcc: cx.source.1,
        fixed: vec![(entered, game.obj(entered).zcc)],
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    game.pending.push(StackItem { delayed: Some(Arc::new(delayed)), x, ..StackItem::spell(cx.source_id(), you) });
    Some(())
}
