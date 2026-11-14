//! Argentum Masticore: "At the beginning of your upkeep, sacrifice Argentum Masticore unless you discard a card. When
//! you discard a card this way, destroy target nonland permanent an opponent controls with mana value less than or
//! equal to the mana value of the discarded card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ArgentumMasticore.ArgentumMasticoreEffect", apply)];

/// "Discard a card?" (not asked with an empty hand); no, or nothing discarded: sacrifice it if it's still there.
/// Otherwise a reflexive trigger destroying a nonland permanent an opponent controls with mana value at most the
/// discarded card's.
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let (src, zcc) = cx.source;
    let still = |game: &Game| game.obj(src).zone == Zone::Battlefield && game.obj(src).zcc == zcc;
    let discarded = if game.player(you).hand.is_empty() || !choose_use(game, d, you, "Discard a card?")? {
        None
    } else {
        let chosen = discard(game, d, you, 1);
        if d.stopped() {
            return None;
        }
        chosen.first().copied()
    };
    let Some(card) = discarded else {
        if still(game) {
            sacrifice(game, src);
        }
        return Some(());
    };
    let mv = game.obj(card).def.mana_value() as i32;
    let target: TargetDef = serde_json::from_value(serde_json::json!({"class": "TargetPermanent", "filter": [
        {"p": "not", "x": {"p": "type", "t": "LAND"}}, {"p": "controller", "who": "opponent"},
        {"p": "mv", "cmp": "FEWER_THAN", "n": mv + 1}]}))
    .ok()?;
    let delayed = Delayed {
        rule: format!("destroy target nonland permanent an opponent controls with mana value {mv} or less."),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![EffectDef::DestroyTargetEffect],
        source_zcc: zcc,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: vec![target],
        condition: None,
    };
    game.pending.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(src, you) });
    Some(())
}
