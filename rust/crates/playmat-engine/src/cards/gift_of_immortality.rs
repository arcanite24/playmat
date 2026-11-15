//! Gift of Immortality: "Enchant creature. When enchanted creature dies, return that card to the battlefield under
//! its owner's control. Return Gift of Immortality to the battlefield attached to that creature at the beginning of
//! the next end step."

use super::*;
use crate::state::Delayed;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("GiftOfImmortality.GiftOfImmortalityEffect", apply),
    // the delayed trigger's `ReturnToBattlefieldAttachedEffect` (no class of the card's)
    Handler::effect("GiftOfImmortality.GiftOfImmortalityReturnAttached", return_attached),
];

/// The creature this Aura enchanted as it left: its card, moved once since (it died), returns under its owner's
/// control; once it's on the battlefield, "at the beginning of the next end step" returns this Aura attached to it
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let aura = game.obj(cx.source_id());
    let on = if aura.zone == Zone::Battlefield && aura.zcc == cx.source.1 { aura.attached_to } else { aura.lki_attached_to };
    let Some((card, zcc)) = on else { return Some(()) };
    let c = game.obj(card);
    if c.token || c.zcc != zcc + 1 || c.zone == Zone::Battlefield {
        return Some(());
    }
    let owner = c.owner;
    put_onto_battlefield(game, d, card, owner);
    if d.stopped() {
        return None;
    }
    if game.obj(card).zone != Zone::Battlefield {
        return Some(());
    }
    let back: EffectDef = serde_json::from_value(
        serde_json::json!({"class": "CardEffect", "handler": "GiftOfImmortality.GiftOfImmortalityReturnAttached"}),
    )
    .ok()?;
    let delayed = Delayed {
        rule: "At the beginning of the next end step, return this card to the battlefield attached to that creature."
            .into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![back],
        source_zcc: cx.source.1,
        fixed: vec![(card, game.obj(card).zcc)],
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    game.at_next_end_step
        .push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), cx.controller()) });
    Some(())
}

/// The creature still on the battlefield and this Aura still in a graveyard: the Aura enters under your control
/// attached to it (`"attachTo:" + auraId`), not asked where
fn return_attached(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let aura = cx.source_id();
    let Some(to) = cx.obj(0) else { return Some(()) };
    if game.obj(aura).zone != Zone::Graveyard || game.obj(to).zone != Zone::Battlefield {
        return Some(());
    }
    game.attach_entering = Some((aura, to, game.obj(to).zcc));
    put_onto_battlefield(game, d, aura, cx.controller());
    game.attach_entering = None;
    if d.stopped() {
        return None;
    }
    Some(())
}
