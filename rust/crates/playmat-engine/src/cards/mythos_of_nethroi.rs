//! Mythos of Nethroi: "Destroy target nonland permanent if it's a creature or if {G}{W} was spent to cast this
//! spell."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MythosOfNethroi.MythosOfNethroiEffect", apply)];

/// `ManaWasSpentCondition.GREEN` and `.WHITE`
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    let (src, you) = (cx.source_id(), cx.controller());
    let spent = |c: Color| game.condition_holds(Some(&Condition::ManaColorSpent { color: c, n: 1 }), you, src);
    if game.has_type(o, CardType::Creature) || (spent(Color::G) && spent(Color::W)) {
        destroy(game, o);
    }
    Some(())
}
