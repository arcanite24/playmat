//! Hexhaven Invigorator: "Whenever this creature is dealt damage, you may search your library for up to that many
//! land cards, put them onto the battlefield tapped, then shuffle."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("HexhavenInvigorator.HexhavenInvigoratorEffect", apply)];

/// `SearchLibraryPutInPlayEffect(TargetCardInLibrary(0, damage, lands), tapped)`: the trigger's X is the damage
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let e = EffectDef::SearchLibraryEffect {
        filter: Filter(vec![Pred::Type { t: CardType::Land }]),
        min: 0,
        max: cx.item.x.max(0) as usize,
        to: crate::card::PutCards::BattlefieldTapped,
        optional: false,
        target_controller: false,
    };
    cx.run(game, d, &[e]);
    Some(())
}
