//! Torgal, A Fine Hound: "Whenever you cast your first Human creature spell each turn, that creature enters with an
//! additional +1/+1 counter on it for each Dog and/or Wolf you control. {T}: Add one mana of any color."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::condition("TorgalAFineHound.TorgalAFineHoundCondition", first_human),
    Handler::replacement("TorgalAFineHound.TorgalAFineHoundEffect", &[EventType::EntersBattlefield], applies, replace)
        .on_start(the_spell),
];

/// `TorgalAFineHoundWatcher`: fewer than two Human creature spells cast by you this turn (this one counted)
fn first_human(game: &Game, you: Seat, _: ObjId) -> bool {
    game.watch.spells.iter().filter(|s| s.caster == you && s.creature && game.has_subtype(s.card, "Human")).count() < 2
}

/// `getValue("spellCast")`: the trigger's spell, kept as the effect's target
fn the_spell(game: &mut Game, _: &mut dyn Decider, _: &StaticCx) -> Option<Vec<(Target, u32)>> {
    let spell = game.resolving_item.as_ref().and_then(|i| i.cast_spell);
    Some(spell.map(|s| (Target::Obj(s), game.obj(s).zcc)).into_iter().collect())
}

/// that spell's card entering, once (`discard()` after it applies)
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let Some(id) = ev.target else { return false };
    cx.left(game, 1) > 0 && matches!(cx.targets.first(), Some((Target::Obj(s), zcc)) if *s == id && game.obj(id).zcc <= zcc + 1)
}

/// a +1/+1 counter, put by you, for each Dog and/or Wolf you control (none: nothing, and it stays)
fn replace(game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    let Some(id) = ev.target else { return false };
    let count = game
        .battlefield
        .iter()
        .filter(|o| {
            game.obj(**o).controller == cx.controller && (game.has_subtype(**o, "Dog") || game.has_subtype(**o, "Wolf"))
        })
        .count() as i32;
    if count < 1 {
        return false;
    }
    add_counters_by(game, id, "+1/+1", count, Some(cx.controller));
    cx.set_left(game, 0);
    false
}
