//! Rushed Rebirth: "Choose target creature. When that creature dies this turn, search your library for a creature card
//! with lesser mana value, put it onto the battlefield tapped, then shuffle."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RushedRebirth.RushedRebirthEffect", apply)];

/// The creature that died (`getFirstTargetPermanentOrLKI`, the delayed trigger's `SetTargetPointer.PERMANENT`): its
/// mana value as it last was, then `SearchLibraryPutInPlayEffect(TargetCardInLibrary(filter), true)` for a creature
/// card with less
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some((died, _)) = cx.item.died else { return Some(()) };
    let mv = game.obj(died).def.mana_value() as i32;
    let e = EffectDef::SearchLibraryEffect {
        filter: Filter(vec![Pred::Type { t: CardType::Creature }, Pred::Mv { cmp: Cmp::FewerThan, n: mv }]),
        min: 1,
        max: 1,
        to: crate::card::PutCards::BattlefieldTapped,
        optional: false,
        target_controller: false,
    };
    cx.run(game, d, &[e]);
    (!d.stopped()).then_some(())
}
