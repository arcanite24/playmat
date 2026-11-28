//! `SpellsCostModificationThatTargetSourceEffect(amount, spellFilter, TargetController.OPPONENT)`: "spells your
//! opponents cast that target this creature cost {N} more (less) to cast" (Icefall Regent, Accursed Witch, Sphinx
//! of New Prahv). An increase for a positive amount, a reduction for a negative one.

use super::*;
use crate::card::ManaCost;

pub const HANDLERS: &[Handler] = &[Handler::cost(
    "SpellsCostModificationThatTargetSourceEffect",
    CostKind::BySign,
    applies,
    apply,
)
.args(&["text", "filter", "one_of:TargetController.OPPONENT"])];

/// `applies`: a spell an opponent casts, of the filter; being cast, with this among its chosen targets; while
/// what's playable is checked, with this among its possible targets (an increase only when it's the only one, as
/// the player could choose another)
fn applies(game: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    if of.ability.is_some() || !cx.opponent(of.seat) || !cx.matches(game, 0, of.card) {
        return false;
    }
    let me = Target::Obj(cx.source);
    if let Some(item) = game.stack.iter().rev().find(|i| i.card == of.card && i.ability.is_none()) {
        return item.targets.iter().flatten().any(|t| *t == me);
    }
    // `CardUtil.getAllPossibleTargets`: every mode's targets
    let spell = &of.face.spell;
    let mut all: Vec<Target> = Vec::new();
    for k in 0..spell.modes.len().max(1) {
        for def in spell.mode(k).1 {
            let (players, objs) = possible_targets(game, of.seat, def, Some(of.card));
            for t in players.into_iter().map(Target::Player).chain(objs.into_iter().map(Target::Obj)) {
                if !all.contains(&t) {
                    all.push(t);
                }
            }
        }
    }
    all.contains(&me) && (cx.arg_int(0).unwrap_or(0) < 0 || all.len() <= 1)
}

fn apply(_: &Game, cx: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    let n = cx.arg_int(0).unwrap_or(0);
    if n >= 0 {
        cost.generic += n as u32;
    } else {
        cost.generic = cost.generic.saturating_sub((-n) as u32);
    }
}
