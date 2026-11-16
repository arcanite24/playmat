//! Invasion of the Giants: "I — Scry 2. II — Draw a card. Then you may reveal a Giant card from your hand. When you
//! do, Invasion of the Giants deals 2 damage to target opponent or planeswalker. III — The next Giant spell you cast
//! this turn costs {2} less to cast."

use super::*;
use crate::card::ManaCost;

pub const HANDLERS: &[Handler] =
    &[Handler::cost("InvasionOfTheGiants.InvasionOfTheGiantsEffect", CostKind::Reduce, next_giant, two_less)
        .on_start(remember)];

/// a Giant (changelings too: every creature type)
fn giant(def: &CardDef) -> bool {
    def.has_subtype("Giant") || def.has(&AbilityDef::ChangelingAbility)
}

/// `InvasionOfTheGiantsWatcher.getCount`: the Giant spells `seat` cast this turn
fn giants_cast(game: &Game, seat: Seat) -> i32 {
    game.watch.spells.iter().filter(|s| s.caster == seat && giant(&game.obj(s.card).def)).count() as i32
}

/// `init`: the Giant spells you've cast so far, kept as the effect's amount
fn remember(game: &mut Game, _: &mut dyn Decider, cx: &StaticCx) -> Option<Vec<(Target, u32)>> {
    let spells = giants_cast(game, cx.controller);
    cx.set_left(game, spells);
    Some(cx.targets.to_vec())
}

/// until another Giant spell is cast after it started (then it's used up): your Giant spells
fn next_giant(game: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    if giants_cast(game, cx.controller) > cx.left(game, 0) {
        return false;
    }
    of.ability.is_none() && of.seat == cx.controller && (giant(of.face) || game.has_subtype(of.card, "Giant"))
}

/// `CardUtil.reduceCost(abilityToModify, 2)`
fn two_less(_: &Game, _: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    cost.generic = cost.generic.saturating_sub(2);
}
