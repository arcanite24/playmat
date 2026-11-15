//! The Celestus: "If it's neither day nor night, it becomes day as The Celestus enters. {T}: Add one mana of any
//! color. {3}, {T}: If it's night, it becomes day. Otherwise, it becomes night. Activate only as a sorcery. Whenever
//! day becomes night or night becomes day, you gain 1 life. You may draw a card. If you do, discard a card."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("TheCelestus.TheCelestusDayNightEffect", day_night_effect),
    Handler::effect("TheCelestus.TheCelestusLootEffect", loot),
];

/// `game.setDaytime(checkDayNight(false))`: night becomes day, anything else night; only a change from day or
/// night fires `BECOMES_DAY_NIGHT` and transforms the daybound and nightbound permanents
fn day_night_effect(game: &mut Game, _d: &mut dyn Decider, _cx: &EffectCx) -> Option<()> {
    let day = game.day == Some(false);
    let changed = game.day.is_some_and(|was| was != day);
    game.day = Some(day);
    if changed {
        let sources = game.trigger_sources();
        let fired = game.triggers(sources, |_, _, _, t| matches!(t, Trigger::DayNightChanges));
        game.pending.extend(fired);
        day_night(game);
    }
    Some(())
}

/// 1 life, then "Draw a card?": a card drawn, then one discarded
fn loot(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    gain_life(game, you, 1);
    if choose_use(game, d, you, "Draw a card?")? {
        draw(game, you, 1);
        discard(game, d, you, 1);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}
