//! PuPu UFO: "Flying. {T}: You may put a land card from your hand onto the battlefield. {3}: Until end of turn, this
//! creature's base power becomes equal to the number of Towns you control."

use super::*;
use crate::card::{Amount, Duration};

pub const HANDLERS: &[Handler] = &[Handler::effect("PuPuUFO.PuPuUFOEffect", apply)];

/// The source still the permanent it was: `SetBasePowerSourceEffect(n, EndOfTurn)`, n the Towns you control as it
/// resolves
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if !cx.source_on_battlefield(game) {
        return Some(());
    }
    let you = cx.controller();
    let towns = game
        .battlefield
        .iter()
        .filter(|o| game.obj(**o).controller == you && game.has_subtype(**o, "Town"))
        .count() as i32;
    let e = EffectDef::SetBasePtEffect {
        power: Some(Amount::Fixed(towns)),
        toughness: None,
        target: false,
        duration: Duration::EndOfTurn,
    };
    cx.run(game, d, &[e]);
    Some(())
}
