//! Leyline Tyrant: "When Leyline Tyrant dies, you may pay any amount of {R}. When you do, it deals that much damage to
//! any target."

use super::*;
use crate::card::ManaCost;

pub const HANDLERS: &[Handler] = &[Handler::effect("LeylineTyrant.LeylineTyrantDamageEffect", apply)];

/// `announceX(0, MAX)`, then that many {R} paid (`{0}` for none); paid, a reflexive "deals X damage to any target"
/// (made even for X = 0, as Java's)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
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
        let mut cost = ManaCost::default();
        cost.colored[3] = x as u32;
        let before = game.clone();
        if !pay_mana(game, d, you, &cost) {
            if d.stopped() {
                return None;
            }
            *game = before;
            return Some(());
        }
    }
    let target: TargetDef = serde_json::from_value(serde_json::json!({"class": "TargetAnyTarget"})).ok()?;
    let delayed = Delayed {
        rule: format!("{{this}} deals {x} damage to any target"),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![EffectDef::DamageTargetEffect { amount: Amount::Fixed(x) }],
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: vec![target],
        condition: None,
    };
    game.pending.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), you) });
    Some(())
}
