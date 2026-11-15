//! Phyrexian Vindicator: "Flying. If damage would be dealt to Phyrexian Vindicator, prevent that damage. When damage
//! is prevented this way, Phyrexian Vindicator deals that much damage to any other target."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "PhyrexianVindicator.PhyrexianVindicatorEffect",
    &[EventType::DamagePermanent],
    applies,
    replace,
)];

/// `PreventDamageToSourceEffect`: preventable damage to this permanent
fn applies(_game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    cx.preventable(ev, false) && ev.target == Some(cx.source)
}

/// All of it prevented; with any prevented, the reflexive trigger "deals that much damage to any other target"
fn replace(game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    let prevented = cx.prevent(ev, i32::MAX);
    if prevented <= 0 {
        return false;
    }
    let Ok(target) = serde_json::from_value::<TargetDef>(serde_json::json!({"class": "TargetAnyTarget", "another": true}))
    else {
        return false;
    };
    let delayed = Delayed {
        rule: format!("{} deals that much damage to any other target", game.obj(cx.source).name()),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![EffectDef::DamageTargetEffect { amount: Amount::Fixed(prevented) }],
        source_zcc: game.obj(cx.source).zcc,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: vec![target],
        condition: None,
    };
    let item = StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source, cx.controller) };
    // (`fireReflexiveTriggeredAbility`: waiting at once, before the triggers of a state-based batch)
    match game.simultaneous_from {
        Some(k) if k <= game.pending.len() => {
            game.pending.insert(k, item);
            game.simultaneous_from = Some(k + 1);
        }
        _ => game.pending.push(item),
    }
    false
}
