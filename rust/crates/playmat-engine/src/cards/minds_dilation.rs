//! Mind's Dilation: "Whenever an opponent casts their first spell each turn, that player exiles the top card of their
//! library. If it's a nonland card, you may cast it without paying its mana cost."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "MindsDilation.MindsDilationTriggeredAbility",
        "new SpellCastOpponentTriggeredAbility(Zone.BATTLEFIELD, $0, StaticFilters.FILTER_SPELL, $1, SetTargetPointer.PLAYER)",
        first_spell,
    ),
    Handler::effect("MindsDilation.MindsDilationEffect", apply),
];

/// `SpellsCastWatcher`: the caster's first spell this turn
fn first_spell(game: &Game, item: &StackItem) -> bool {
    let Some(s) = item.cast_spell else { return false };
    game.player(game.obj(s).controller).spells_cast == 1
}

/// that player exiles their top card; a nonland card: "Cast <card> without paying its mana cost from exile?", then
/// the cast of the face `chooseAbilityForCast(card, game, true)` picks
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(p) = cx.player(0) else { return Some(()) };
    let Some(card) = game.player(p).library.first().copied() else { return Some(()) };
    game.move_to(card, Zone::Exile);
    if game.obj(card).zone != Zone::Exile || game.obj(card).is(CardType::Land) {
        return Some(());
    }
    let message = format!("Cast {} without paying its mana cost from exile?", log::object(game, card));
    if choose_use(game, d, you, &message)? {
        cast_free_face(game, d, you, card);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}
