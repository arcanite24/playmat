//! Coalstoke Gearhulk: "When this creature enters, return target creature card with mana value 4 or less from a
//! graveyard to the battlefield under your control with a finality counter on it. That creature gains menace,
//! deathtouch, and haste. At the beginning of your next end step, exile that creature."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("CoalstokeGearhulk.CoalstokeGearhulkEffect", apply)];

/// The returned creature, while it's still that permanent (`FixedTarget(permanent, game)`), exiled at your next end
/// step; the delayed trigger's text is the effect's own, after the trigger phrase, with the creature's log name
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.obj(0)?;
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let rule = format!(
        "At the beginning of your next end step, At the beginning of your next end step, exile {}.",
        crate::log::object(game, o)
    );
    let delayed = crate::state::Delayed {
        rule,
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![EffectDef::ExileTargetEffect],
        source_zcc: cx.source.1,
        fixed: vec![(o, game.obj(o).zcc)],
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    game.at_your_next_end_step
        .push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), cx.controller()) });
    Some(())
}
