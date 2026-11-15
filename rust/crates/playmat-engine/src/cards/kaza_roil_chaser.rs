//! Kaza, Roil Chaser: "{T}: The next instant or sorcery spell you cast this turn costs {X} less to cast, where X is
//! the number of Wizards you control as this ability resolves."

use super::*;
use crate::card::ManaCost;

pub const HANDLERS: &[Handler] =
    &[Handler::cost("KazaRoilChaser.KazaRoilChaserEffect", CostKind::Reduce, next_instant_or_sorcery, x_less)
        .on_start(remember)];

/// `KazaRoilChaserWatcher.getCount`: the instant and sorcery spells `seat` cast this turn
fn instants_sorceries_cast(game: &Game, seat: Seat) -> i32 {
    game.watch
        .spells
        .iter()
        .filter(|s| {
            let def = &game.obj(s.card).def;
            s.caster == seat && (def.is(CardType::Instant) || def.is(CardType::Sorcery))
        })
        .count() as i32
}

/// `init`: the Wizards you control and the instants and sorceries you've cast so far, kept as the effect's amount
/// (Wizards × 1000 + spells)
fn remember(game: &mut Game, _: &mut dyn Decider, cx: &StaticCx) -> Option<Vec<(Target, u32)>> {
    let wizards = game
        .battlefield
        .iter()
        .filter(|o| game.obj(**o).controller == cx.controller && game.has_subtype(**o, "Wizard"))
        .count() as i32;
    let spells = instants_sorceries_cast(game, cx.controller);
    cx.set_left(game, wizards * 1000 + spells);
    Some(cx.targets.to_vec())
}

/// until another instant or sorcery is cast after it started (then it's used up): your instant and sorcery spells
fn next_instant_or_sorcery(game: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    let kept = cx.left(game, 0);
    if instants_sorceries_cast(game, cx.controller) > kept % 1000 {
        return false;
    }
    of.ability.is_none()
        && of.seat == cx.controller
        && (of.face.is(CardType::Instant) || of.face.is(CardType::Sorcery))
}

/// `CardUtil.reduceCost(ability, wizardCount)`: that much generic mana
fn x_less(game: &Game, cx: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    let wizards = (cx.left(game, 0) / 1000).max(0) as u32;
    cost.generic = cost.generic.saturating_sub(wizards);
}
