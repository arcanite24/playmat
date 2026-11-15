//! Dark Intimations: "Each opponent sacrifices a creature or planeswalker, then discards a card. You return a
//! creature or planeswalker card from your graveyard to your hand, then draw a card. When you cast a Bolas
//! planeswalker spell, exile Dark Intimations from your graveyard. That planeswalker enters with an additional
//! loyalty counter on it."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("DarkIntimations.DarkIntimationsGraveyardEffect", apply)
        .effects(&["new DarkIntimationsReplacementEffect()"]),
    Handler::replacement(
        "DarkIntimations.DarkIntimationsReplacementEffect",
        &[EventType::EntersBattlefield],
        applies,
        replace,
    ),
];

/// This card exiled from your graveyard (if it's still there); then, with the spell still on the stack, the
/// replacement for it (`FixedTarget(spell.getSourceId())`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let me = cx.source_id();
    if game.obj(me).zone == Zone::Graveyard && game.obj(me).owner == cx.controller() {
        game.move_to(me, Zone::Exile);
    }
    let Some(spell) = cx.obj(0) else { return Some(()) };
    if game.obj(spell).zone == Zone::Stack {
        cx.run_on(game, d, &cx.effects[..1], vec![Target::Obj(spell)]);
    }
    Some(())
}

/// that card entering (by its id: the pointer has no zone-change count), once (`Duration.OneUse`)
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let Some(id) = ev.target else { return false };
    cx.left(game, 1) > 0 && matches!(cx.targets.first(), Some((Target::Obj(o), _)) if *o == id)
}

/// a loyalty counter put on it by you, then the effect is used up
fn replace(game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    if let Some(id) = ev.target {
        add_counters_by(game, id, "loyalty", 1, Some(cx.controller));
        cx.set_left(game, 0);
    }
    false
}
