//! Psychic Rebuttal: "Counter target instant or sorcery spell that targets you. Spell mastery — If there are two or
//! more instant and/or sorcery cards in your graveyard, you may copy the spell countered this way. You may choose
//! new targets for the copy."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::predicate("PsychicRebuttal.PsychicRebuttalPredicate", targets_you),
    Handler::effect("PsychicRebuttal.PsychicRebuttalEffect", apply).effects(&["new CounterTargetEffect()"]),
];

/// The spell's targets include you (the player choosing)
fn targets_you(game: &Game, you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    game.stack
        .iter()
        .rev()
        .find(|i| i.card == id && i.ability.is_none())
        .is_some_and(|i| i.targets.iter().flatten().any(|t| *t == Target::Player(you)))
}

/// the spell countered; then, with spell mastery, "Copy <name> (you may choose new targets for the copy)?" and a
/// copy of it for you
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(spell) = cx.obj(0) else { return Some(()) };
    let Some(original) =
        game.stack.iter().find(|i| i.card == spell && i.ability.is_none() && i.delayed.is_none()).cloned()
    else {
        return Some(());
    };
    cx.run(game, d, &cx.effects[..1]);
    if d.stopped() {
        return None;
    }
    let you = cx.controller();
    let spells = game
        .player(you)
        .graveyard
        .iter()
        .filter(|c| game.obj(**c).is(CardType::Instant) || game.obj(**c).is(CardType::Sorcery))
        .count();
    if spells < 2 {
        return Some(());
    }
    let message = format!("Copy {} (you may choose new targets for the copy)?", game.obj(spell).name());
    if choose_use(game, d, you, &message)? {
        copy_spell_with(game, d, &original, you, true, false)?;
    }
    Some(())
}
