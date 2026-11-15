//! Necromantic Summons: "Put target creature card from a graveyard onto the battlefield under your control. Spell
//! mastery — If there are two or more instant and/or sorcery cards in your graveyard, that creature enters the
//! battlefield with two additional +1/+1 counters on it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "NecromanticSummons.NecromanticSummoningReplacementEffect",
    &[EventType::EntersBattlefield],
    applies,
    replace,
)];

/// the spell's target entering, with spell mastery (two or more instant and/or sorcery cards in your graveyard);
/// once (`discard()` after it applies)
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let Some(id) = ev.target else { return false };
    if cx.left(game, 1) <= 0 || !matches!(cx.targets.first(), Some((Target::Obj(o), _)) if *o == id) {
        return false;
    }
    let spells = game
        .player(cx.controller)
        .graveyard
        .iter()
        .filter(|c| game.has_type(**c, CardType::Instant) || game.has_type(**c, CardType::Sorcery))
        .count();
    spells >= 2
}

/// two +1/+1 counters put on it by you, then the effect is done
fn replace(game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    if let Some(id) = ev.target {
        add_counters_by(game, id, "+1/+1", 2, Some(cx.controller));
        cx.set_left(game, 0);
    }
    false
}
