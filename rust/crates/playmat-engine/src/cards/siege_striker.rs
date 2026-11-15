//! Siege Striker: "Whenever Siege Striker attacks, you may tap any number of untapped creatures you control. Siege
//! Striker gets +1/+1 until end of turn for each creature tapped this way."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SiegeStriker.SiegeStrikerEffect", apply)
    .effects(&["new BoostSourceEffect(1, 1, Duration.EndOfTurn)"])];

/// `TargetPermanent(0, MAX, FILTER_CONTROLLED_UNTAPPED_CREATURES, true).choose(Tap, …)`: rounds, then the taps and
/// the boost for each one tapped
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let possible: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == you && game.has_type(*o, CardType::Creature) && !game.obj(*o).tapped)
        .collect();
    let chosen = choose_in_rounds(game, d, you, &possible, 0, possible.len(), "tap")?;
    let mut n = 0;
    for o in chosen {
        if game.obj(o).zone == Zone::Battlefield && !game.obj(o).tapped {
            tap(game, o);
            n += 1;
        }
    }
    if n > 0 {
        let mut e = cx.effects[0].clone();
        if let EffectDef::BoostSourceEffect { power, toughness, .. } = &mut e {
            *power = Amount::Fixed(n);
            *toughness = Amount::Fixed(n);
        }
        cx.run(game, d, &[e]);
    }
    (!d.stopped()).then_some(())
}
