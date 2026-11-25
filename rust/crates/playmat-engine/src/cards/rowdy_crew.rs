//! Rowdy Crew: "When Rowdy Crew enters the battlefield, draw three cards, then discard two cards at random. If two
//! cards that share a card type are discarded this way, put two +1/+1 counters on Rowdy Crew."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RowdyCrew.RowdyCrewEffect", apply)];

const TYPES: [CardType; 9] = [
    CardType::Artifact,
    CardType::Battle,
    CardType::Creature,
    CardType::Enchantment,
    CardType::Instant,
    CardType::Kindred,
    CardType::Land,
    CardType::Planeswalker,
    CardType::Sorcery,
];

/// Draw three; with one card in hand it's discarded at random, with more two are; two discarded that share a card
/// type put two +1/+1 counters on the permanent (`game.getPermanent(sourceId)`, looked up first)
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let me = cx.source_id();
    let creature = game.obj(me).zone == Zone::Battlefield;
    draw(game, you, 3);
    let in_hand = game.player(you).hand.len();
    let discarded = match in_hand {
        0 => Vec::new(),
        1 => {
            discard_at_random(game, you, 1);
            Vec::new()
        }
        _ => discard_at_random(game, you, 2),
    };
    if creature && in_hand > 1 && discarded.len() == 2 {
        let share = TYPES.iter().any(|t| discarded.iter().all(|c| game.has_type(*c, *t)));
        if share {
            add_counters_by(game, me, "+1/+1", 2, Some(you));
        }
    }
    Some(())
}
