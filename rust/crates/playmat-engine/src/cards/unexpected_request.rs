//! Unexpected Request: "Gain control of target creature until end of turn. Untap that creature. It gains haste
//! until end of turn. You may attach an Equipment you control to that creature. If you do, unattach it at the
//! beginning of the next end step."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("UnexpectedRequest.UnexpectedRequestAttachEffect", apply)
        .effects(&["new UnexpectedRequestUnattachEffect(equipment, game)"]),
    Handler::effect("UnexpectedRequest.UnexpectedRequestUnattachEffect", unattach).args(&["text", "text"]),
];

/// The target still on the battlefield and an Equipment you control (else nothing): you choose up to one
/// (`TargetPermanent(0, 1, …, true)` with `choose`); attached (`addAttachment`: not one already attached to it),
/// a delayed "unattach that equipment" at the beginning of the next end step points at it
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(creature) = cx.obj(0).filter(|o| game.obj(*o).zone == Zone::Battlefield) else { return Some(()) };
    let equipment: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == you && game.has_subtype(*o, "Equipment"))
        .collect();
    if equipment.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let Some(&e) = choose_cards(game, d, you, &equipment, &equipment, 0, 1, &rule)?.first() else {
        return Some(());
    };
    if game.obj(e).zone != Zone::Battlefield
        || e == creature
        || game.attached(e) == Some(creature)
        || game.has_type(e, CardType::Creature)
        || game.protected_from(creature, e)
    {
        return Some(());
    }
    let z = game.obj(creature).zcc;
    game.attach_to(e, creature, z);
    let delayed = crate::state::Delayed {
        rule: "At the beginning of the next end step, unattach that equipment.".into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: cx.effects[..1].to_vec(),
        source_zcc: cx.source.1,
        fixed: vec![(e, game.obj(e).zcc)],
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    game.at_next_end_step
        .push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), cx.controller()) });
    Some(())
}

/// The Equipment, still on the battlefield, becomes unattached (`Permanent.unattach`)
fn unattach(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(e) = cx.obj(0).filter(|o| game.obj(*o).zone == Zone::Battlefield) else { return Some(()) };
    if game.obj(e).attached_to.is_some() {
        game.obj_mut(e).attached_to = None;
        refresh_becomes(game);
    }
    Some(())
}
