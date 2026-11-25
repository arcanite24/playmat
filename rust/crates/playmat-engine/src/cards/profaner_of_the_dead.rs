//! Profaner of the Dead: "Exploit. When Profaner of the Dead exploits a creature, return to their owners' hands all
//! creatures your opponents control with toughness less than the exploited creature's toughness."

use super::*;
use crate::card::{Cmp, Filter, Pred};

pub const HANDLERS: &[Handler] =
    &[Handler::effect("ProfanerOfTheDead.ProfanerOfTheDeadReturnEffect", apply)];

/// The exploited creature (the trigger's pointer, `getFirstTargetPermanentOrLKI`): its toughness as it last was on
/// the battlefield; the opponents' creatures with less go to their owners' hands, in battlefield order
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(Target::Obj(o)) = cx.item.targets.first().and_then(|ts| ts.first()).copied() else { return Some(()) };
    let ob = game.obj(o);
    let toughness = if ob.zone == Zone::Battlefield { game.power_toughness(o).1 } else { ob.toughness_lki? };
    let filter = Filter(vec![
        Pred::Type { t: CardType::Creature },
        Pred::Controller { who: "opponent".into() },
        Pred::Toughness { cmp: Cmp::FewerThan, n: toughness },
    ]);
    cx.run(game, d, &[EffectDef::ReturnToHandFromBattlefieldAllEffect { filter }]);
    Some(())
}
