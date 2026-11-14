//! Hateful Eidolon: "Whenever an enchanted creature dies, draw a card for each Aura you controlled that was attached
//! to it."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "HatefulEidolon.HatefulEidolonTriggeredAbility",
        "new DiesCreatureTriggeredAbility(Zone.BATTLEFIELD, new HatefulEidolonDrawEffect(), false, StaticFilters.FILTER_PERMANENT, SetTargetPointer.PERMANENT)",
        had_your_auras,
    ),
    Handler::effect("HatefulEidolon.HatefulEidolonDrawEffect", draw_for_each),
];

/// the enchantments you controlled that were attached to the permanent that died, as it last was (Shadowspear or
/// another Equipment doesn't count)
fn auras(game: &Game, you: Seat, dead: ObjId) -> usize {
    game.obj(dead)
        .lki_attachments
        .iter()
        .filter(|(a, _)| game.obj(*a).controller == you && game.has_type(*a, CardType::Enchantment))
        .count()
}

fn dead(t: &StackItem) -> Option<ObjId> {
    match t.targets.first().and_then(|g| g.first()) {
        Some(Target::Obj(o)) => Some(*o),
        _ => None,
    }
}

/// `checkTrigger`: one or more of your Auras were attached to it
fn had_your_auras(game: &Game, t: &StackItem) -> bool {
    dead(t).is_some_and(|o| auras(game, t.controller, o) > 0)
}

/// `DrawCardTargetEffect(auraCount)` for you
fn draw_for_each(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let n = dead(cx.item).map_or(0, |o| auras(game, you, o));
    if n > 0 {
        draw(game, you, n);
    }
    Some(())
}
