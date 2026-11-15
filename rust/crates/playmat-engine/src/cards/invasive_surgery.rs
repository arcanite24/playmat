//! Invasive Surgery: "Counter target sorcery spell. Delirium — If there are four or more card types among cards in
//! your graveyard, search the graveyard, hand, and library of that spell's controller for any number of cards with
//! the same name as that spell, exile those cards, then that player shuffles."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("InvasiveSurgery.InvasiveSurgeryEffect", apply).effects(&["new CounterTargetEffect()"])];

/// The spell's name and controller as it's countered; then, with delirium, `applySearchAndExile` for that player
/// (the graveyard's choice optional, any number)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let spell = cx.obj(0).and_then(|o| {
        game.stack.iter().find(|i| i.card == o && i.ability.is_none()).map(|i| (game.obj(o).def.name.clone(), i.controller))
    });
    if spell.is_some() {
        cx.run(game, d, &cx.effects[..1]);
        if d.stopped() {
            return None;
        }
    }
    let you = cx.controller();
    if !game.condition_holds(Some(&crate::card::Condition::Delirium), you, cx.source_id()) {
        return Some(());
    }
    let Some((name, controller)) = spell else { return Some(()) };
    super::search_for_card_name_and_exile::search_and_exile_named(game, d, you, controller, &name, usize::MAX, false)
}
