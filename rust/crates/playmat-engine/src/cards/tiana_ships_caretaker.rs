//! Tiana, Ship's Caretaker: "Whenever an Aura or Equipment you control is put into a graveyard from the battlefield,
//! you may return that card to its owner's hand at the beginning of the next end step."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "TianaShipsCaretaker.TianaShipsCaretakerTriggeredAbility",
        "new DiesCreatureTriggeredAbility(Zone.BATTLEFIELD, new TianaShipsCaretakerEffect(), true, StaticFilters.FILTER_CONTROLLED_PERMANENT, SetTargetPointer.PERMANENT)",
        aura_or_equipment,
    ),
    Handler::effect("TianaShipsCaretaker.TianaShipsCaretakerEffect", return_later)
        .effects(&["new ReturnFromGraveyardToHandTargetEffect()"]),
];

/// the card it became was an Equipment artifact or an Aura enchantment
fn aura_or_equipment(game: &Game, t: &StackItem) -> bool {
    match t.targets.first().and_then(|g| g.first()) {
        Some(Target::Obj(o)) => {
            (game.has_type(*o, CardType::Artifact) && game.has_subtype(*o, "Equipment"))
                || (game.has_type(*o, CardType::Enchantment) && game.has_subtype(*o, "Aura"))
        }
        _ => false,
    }
}

/// a delayed "return that card" while it's still that card in the graveyard
fn return_later(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(c) = cx.obj(0) else { return Some(()) };
    if game.obj(c).zone != Zone::Graveyard {
        return Some(());
    }
    let delayed = Delayed {
        rule: "At the beginning of the next end step, return that card to your hand at the beginning of the next end step."
            .into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: cx.effects.to_vec(),
        source_zcc: game.obj(cx.source_id()).zcc,
        fixed: vec![(c, game.obj(c).zcc)],
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    let you = cx.controller();
    game.at_next_end_step.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), you) });
    Some(())
}
