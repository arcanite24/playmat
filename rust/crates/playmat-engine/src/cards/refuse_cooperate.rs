//! Refuse // Cooperate: "Refuse deals damage to target spell's controller equal to that spell's mana value." //
//! "Aftermath. Copy target instant or sorcery spell. You may choose new targets for the copy."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RefuseCooperate.RefuseEffect", apply)];

/// The target spell still on the stack: its mana value (X included, `Spell.getManaValue`) as damage to its
/// controller
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(spell) = cx.obj(0).filter(|o| game.obj(*o).zone == Zone::Stack) else { return Some(()) };
    let Some(item) = game.stack.iter().find(|t| t.card == spell && t.ability.is_none()) else { return Some(()) };
    let (who, x) = (item.controller, item.x);
    let def = &game.obj(spell).def;
    let mv = def.mana_value() as i32 + x * def.cost.x as i32;
    deal_damage(game, cx.source_id(), Target::Player(who), mv, false);
    Some(())
}
