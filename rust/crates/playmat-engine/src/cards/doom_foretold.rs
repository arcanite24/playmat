//! Doom Foretold: "At the beginning of each player's upkeep, that player sacrifices a nonland, nontoken permanent. If
//! that player can't, they discard a card, they lose 2 life, you draw a card, you gain 2 life, you create a 2/2 white
//! Knight creature token with vigilance, then you sacrifice Doom Foretold."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DoomForetold.DoomForetoldEffect", apply)
    .effects(&["new CreateTokenEffect(new KnightToken())", "new SacrificeSourceEffect()"])];

/// The active player's `TargetSacrifice` (picked for them when only one fits); with none sacrificed, the rest in
/// order
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let p = game.active?;
    let possible: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == p && !game.obj(*o).token && !game.has_type(*o, CardType::Land))
        .collect();
    if !possible.is_empty() {
        let chosen = choose_cards(game, d, p, &possible, &possible, 1, 1, "sacrifice")?;
        if let Some(&o) = chosen.first() {
            if game.obj(o).zone == Zone::Battlefield {
                sacrifice(game, o);
                if game.obj(o).zone != Zone::Battlefield {
                    return Some(());
                }
            }
        }
    }
    discard(game, d, p, 1);
    if d.stopped() {
        return None;
    }
    lose_life(game, p, 2);
    draw(game, you, 1);
    gain_life(game, you, 2);
    cx.run(game, d, cx.effects);
    (!d.stopped()).then_some(())
}
