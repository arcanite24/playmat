//! Rescue from the Underworld: "As an additional cost to cast this spell, sacrifice a creature. Choose target
//! creature card in your graveyard. Return that card and the sacrificed card to the battlefield under your control at
//! the beginning of your next upkeep. Exile Rescue from the Underworld."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("RescueFromTheUnderworld.RescueFromTheUnderworldTextEffect", text),
    Handler::effect("RescueFromTheUnderworld.RescueFromTheUnderworldCreateDelayedTriggeredAbilityEffect", apply),
    Handler::effect("RescueFromTheUnderworld.RescueFromTheUnderworldReturnEffect", back),
];

/// "Choose target creature card in your graveyard": the target does the choosing
fn text(_: &mut Game, _: &mut dyn Decider, _: &EffectCx) -> Option<()> {
    Some(())
}

/// The delayed trigger at the beginning of your next upkeep, holding the target (as it is now) and then the
/// permanents the sacrifice cost sacrificed (by card, whatever happens to them)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    // (the spell's only target: without it, the spell doesn't resolve)
    let Some(target) = cx.obj(0) else { return Some(()) };
    let fixed: Vec<(ObjId, u32)> =
        std::iter::once(target).chain(cx.item.sacrificed.iter().copied()).map(|c| (c, game.obj(c).zcc)).collect();
    let back: EffectDef = serde_json::from_value(serde_json::json!({"class": "CardEffect",
        "handler": "RescueFromTheUnderworld.RescueFromTheUnderworldReturnEffect"}))
    .ok()?;
    let delayed = Delayed {
        rule: "Return that card and the sacrificed card to the battlefield under your control at the beginning of \
               your next upkeep"
            .into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![back],
        source_zcc: cx.source.1,
        fixed,
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    game.at_first_upkeep
        .push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), cx.controller()) });
    Some(())
}

/// The target card, if it's still the card that was targeted, onto the battlefield under your control; then the
/// sacrificed card, if it's a card in a graveyard or exile now, under its owner's control (`player.moveCards`)
fn back(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(dl) = cx.item.delayed.as_ref() else { return Some(()) };
    let Some((&(c, zcc), rest)) = dl.fixed.split_first() else { return Some(()) };
    let o = game.obj(c);
    if o.zcc == zcc && !o.token && o.zone != Zone::Battlefield {
        enter_battlefield(game, d, c, you, false);
        if d.stopped() {
            return None;
        }
    }
    // (only the first sacrificed card the loop comes to that's in one of those zones: `return player.moveCards`)
    for &(c, _) in rest {
        let o = game.obj(c);
        if o.token || o.face_down || !matches!(o.zone, Zone::Graveyard | Zone::Exile | Zone::Command) {
            continue;
        }
        let owner = o.owner;
        enter_battlefield(game, d, c, owner, false);
        return (!d.stopped()).then_some(());
    }
    Some(())
}
