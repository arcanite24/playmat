//! Magma Pummeler: "Magma Pummeler enters the battlefield with X +1/+1 counters on it. If damage would be dealt to
//! Magma Pummeler while it has a +1/+1 counter on it, prevent that damage and remove that many +1/+1 counters from
//! it. When one or more counters are removed from Magma Pummeler this way, it deals that much damage to any target."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "MagmaPummeler.MagmaPummelerPreventionEffect",
    &[EventType::DamagePermanent],
    applies,
    replace,
)];

const RULE: &str = "When one or more counters are removed from {this} this way, it deals that much damage to any target.";

/// `PreventDamageAndRemoveCountersEffect(true, true, true).applies`: damage to it that can be prevented, while it
/// has a +1/+1 counter (or it already removed some in this damage batch)
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    if !cx.preventable(ev, false) || ev.target != Some(cx.source) || game.obj(cx.source).zone != Zone::Battlefield {
        return false;
    }
    game.obj(cx.source).counter("+1/+1") > 0 || game.counter_removed_batch.contains(&cx.source)
}

/// All of it prevented and that many counters removed; the first removal of a damage batch fires the reflexive
/// trigger (`fireReflexiveTriggeredAbility`), and later ones in the same batch raise its "that much" to the batch's
/// total while it waits (`onDamagePrevented`)
fn replace(game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    let amount = ev.amount;
    cx.prevent(ev, i32::MAX);
    if amount <= 0 {
        return false;
    }
    let first = !game.counter_removed_batch.contains(&cx.source);
    let o = game.obj_mut(cx.source);
    let n = amount.min(o.counter("+1/+1"));
    if n > 0 {
        o.add_counters("+1/+1", -n);
        o.counters.retain(|(_, k)| *k > 0);
    }
    if first {
        game.counter_removed_batch.push(cx.source);
    }
    let waiting = game.pending.iter().position(|t| {
        t.card == cx.source && t.delayed.as_ref().is_some_and(|d| d.rule == RULE && d.source_zcc == game.obj(cx.source).zcc)
    });
    let earlier = |t: &StackItem| match t.delayed.as_ref().and_then(|d| d.effects.first()) {
        Some(EffectDef::DamageTargetEffect { amount: Amount::Fixed(k) }) => *k,
        _ => 0,
    };
    if first && n > 0 {
        let delayed = reflexive(game, cx, n);
        game.pending.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source, cx.controller) });
    } else if let (false, Some(at)) = (first, waiting) {
        let total = earlier(&game.pending[at]) + n;
        let delayed = reflexive(game, cx, total);
        game.pending[at].delayed = Some(Arc::new(delayed));
    }
    false
}

fn reflexive(game: &Game, cx: &StaticCx, much: i32) -> Delayed {
    Delayed {
        rule: RULE.into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![EffectDef::DamageTargetEffect { amount: Amount::Fixed(much) }],
        source_zcc: game.obj(cx.source).zcc,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: vec![TargetDef::TargetAnyTarget {
            min: 1,
            max: 1,
            another: false,
            not_first: false,
            distribute: None,
            x_count: false,
            count: None,
        }],
        condition: None,
    }
}
