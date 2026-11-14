//! Domri, Anarch of Bolas: "+1: Add {R} or {G}. Creature spells you cast this turn can't be countered."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DomriAnarchOfBolas.DomriAnarchOfBolasEffect", apply)
    .filters(&["StaticFilters.FILTER_SPELL_A_CREATURE"])];

/// `chooseUse("Choose a color of mana to add", "Red", "Green")`: yes is red; then `CantBeCounteredControlledEffect`
/// until end of turn
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let red = choose_use(game, d, you, "Choose a color of mana to add")?;
    let c = if red { Color::R } else { Color::G };
    game.player_mut(you).mana_pool.colored[c.index()] += 1;
    let e = EffectDef::CantBeCounteredControlledEffect {
        filter: cx.filters[0].clone(),
        duration: crate::card::Duration::EndOfTurn,
    };
    game.eot_effects.push((e, you, cx.source_id()));
    Some(())
}
