//! Alluring Suitor // Deadly Dancer: "When you attack with exactly two creatures, transform Alluring Suitor."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "AlluringSuitor.AlluringSuitorTriggeredAbility",
    "new AttacksWithCreaturesTriggeredAbility(new TransformSourceEffect(), 2)",
    check,
)];

/// `DECLARED_ATTACKERS` by you (the base trigger, two or more attackers): exactly two attacking creatures
fn check(game: &Game, _t: &StackItem) -> bool {
    attackers(game).iter().filter(|o| game.has_type(**o, CardType::Creature)).count() == 2
}
