//! Path to the World Tree: "When Path to the World Tree enters, you may search your library for a basic land card,
//! reveal it, put it into your hand, then shuffle. {2}{W}{U}{B}{R}{G}, Sacrifice Path to the World Tree: You gain 2
//! life and draw two cards. Target opponent loses 2 life. Path to the World Tree deals 2 damage to up to one target
//! creature. You create a 2/2 green Bear creature token."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("PathToTheWorldTree.PathToTheWorldTreeEffect", apply).tokens(&["BearToken"])];

/// you gain 2 and draw two; each target in order: a player loses 2, a creature is dealt 2 by Path; then a Bear
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    gain_life(game, you, 2);
    draw(game, you, 2);
    let targets: Vec<Target> = cx.legal.iter().flatten().copied().collect();
    for t in targets {
        match t {
            Target::Player(p) => lose_life(game, p, 2),
            Target::Obj(o) => {
                if game.obj(o).zone == Zone::Battlefield {
                    deal_damage(game, cx.source_id(), Target::Obj(o), 2, false);
                }
            }
        }
    }
    cx.create_tokens(game, d, &cx.tokens[0], 1);
    Some(())
}
