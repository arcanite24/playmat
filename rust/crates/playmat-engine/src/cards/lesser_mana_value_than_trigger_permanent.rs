//! "creature card with lesser mana value" than the permanent a trigger is about: Orah, Skyclave Hierophant ("Whenever
//! Orah or another Cleric you control dies, return target Cleric card with lesser mana value from your graveyard to
//! the battlefield", `getValue("creatureDied")`) and Riveteers Ascendancy ("Whenever you sacrifice a creature,
//! you may return target creature card with lesser mana value from your graveyard to the battlefield", once each
//! turn, `getValue("sacrificedPermanent")`).

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::predicate("OrahSkyclaveHierophant.OrahSkyclaveHierophantPredicate", lesser_than_died),
    Handler::predicate("RiveteersAscendancy.RiveteersAscendancyPredicate", lesser_than_sacrificed),
];

/// The trigger whose targets are being chosen (on top of the stack) or checked as it resolves, from `source`
fn trigger_item(game: &Game, source: Option<ObjId>) -> Option<&StackItem> {
    let src = source?;
    game.resolving_item
        .as_ref()
        .filter(|i| i.card == src && i.ability.is_some())
        .or_else(|| game.stack.last().filter(|i| i.card == src && i.ability.is_some()))
}

fn lesser(game: &Game, than: Option<ObjId>, id: ObjId) -> bool {
    than.is_some_and(|p| game.obj(id).def.mana_value() < game.obj(p).def.mana_value())
}

fn lesser_than_died(game: &Game, _: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    lesser(game, trigger_item(game, source).and_then(|i| i.died.map(|d| d.0)), id)
}

fn lesser_than_sacrificed(game: &Game, _: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    lesser(game, trigger_item(game, source).and_then(|i| i.sacrificed_permanent), id)
}
