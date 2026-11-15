//! Zenith Chronicler: "Whenever a player casts their first multicolored spell each turn, each other player draws a
//! card."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "ZenithChronicler.ZenithChroniclerTriggeredAbility",
        "new SpellCastAllTriggeredAbility(new ZenithChroniclerEffect(), StaticFilters.FILTER_SPELL_A_MULTICOLORED, false, SetTargetPointer.PLAYER)",
        first_multicolored,
    ),
    Handler::effect("ZenithChronicler.ZenithChroniclerEffect", apply),
];

/// `SpellsCastWatcher`: up to the spell that triggered it, its caster cast one multicolored spell this turn (read
/// up to that spell, so it holds the same as the trigger resolves; the earlier spells' colors are read as they are
/// now)
fn first_multicolored(game: &Game, t: &StackItem) -> bool {
    let Some(spell) = t.cast_spell else { return false };
    let Some(at) = game.watch.spells.iter().rposition(|s| s.card == spell) else { return false };
    let caster = game.watch.spells[at].caster;
    game.watch.spells[..=at].iter().filter(|s| s.caster == caster && game.colors(s.card).len() > 1).count() == 1
}

/// each opponent of the player who cast it draws a card
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = cx.player(0) else { return Some(()) };
    let n = game.seats();
    for i in 1..n {
        let o = (p + i) % n;
        if !game.player(o).lost {
            draw(game, o, 1);
        }
    }
    Some(())
}
