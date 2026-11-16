//! Dire-Strain Rampage: "Destroy target artifact, enchantment, or land. If a land was destroyed this way, its
//! controller may search their library for up to two basic land cards, put them onto the battlefield tapped, then
//! shuffle. Otherwise, its controller may search their library for a basic land card, put it onto the battlefield
//! tapped, then shuffle. Flashback {3}{R}{G}"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DireStrainRampage.DireStrainRampageEffect", apply)
    .filters(&["StaticFilters.FILTER_CARD_BASIC_LAND"])];

/// The target destroyed (a land, and destroyed: `TargetCardInLibrary(0, 2)`, else `(1, 1)`), then
/// `SearchLibraryPutInPlayTargetControllerEffect(target, tapped)` for its controller (or last controller)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.obj(0).filter(|o| game.obj(*o).zone == Zone::Battlefield)?;
    let land = game.has_type(o, CardType::Land);
    destroy(game, o);
    let destroyed = game.obj(o).zone != Zone::Battlefield;
    let (min, max) = if land && destroyed { (0, 2) } else { (1, 1) };
    let e = EffectDef::SearchLibraryEffect {
        filter: cx.filters[0].clone(),
        min,
        max,
        to: crate::card::PutCards::BattlefieldTapped,
        optional: false,
        target_controller: true,
    };
    cx.run_on(game, d, &[e], vec![Target::Obj(o)]);
    (!d.stopped()).then_some(())
}
