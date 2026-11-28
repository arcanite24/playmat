//! Vance's Blasting Cannons: "At the beginning of your upkeep, exile the top card of your library. If it's a nonland
//! card, you may cast that card this turn. Whenever you cast your third spell in a turn, you may transform Vance's
//! Blasting Cannons."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("VancesBlastingCannons.VancesBlastingCannonsExileEffect", exile_top),
    Handler::trigger(
        "VancesBlastingCannons.VancesBlastingCannonsTriggeredAbility",
        "new SpellCastControllerTriggeredAbility(new TransformSourceEffect(), true)",
        third_spell,
    ),
];

/// Into exile; a nonland card may be cast this turn (`makeCardPlayable`, not played as a land)
fn exile_top(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(card) = game.player(you).library.first().copied() else { return Some(()) };
    game.move_to(card, Zone::Exile);
    if game.obj(card).zone == Zone::Exile && !game.has_type(card, CardType::Land) {
        let turn = game.turn;
        let o = game.obj_mut(card);
        o.may_play = Some((you, turn));
        o.cast_only = true;
    }
    Some(())
}

/// The controller's third spell this turn
fn third_spell(game: &Game, t: &StackItem) -> bool {
    game.player(t.controller).spells_cast == 3
}
