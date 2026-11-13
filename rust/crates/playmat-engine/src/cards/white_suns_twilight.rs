//! White Sun's Twilight: "You gain X life. Create X 1/1 colorless Phyrexian Mite artifact creature tokens with
//! toxic 1 and "This creature can't block." If X is 5 or more, destroy all other creatures."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("WhiteSunsTwilight.WhiteSunsTwilightEffect", apply)
    .effects(&["new CreateTokenEffect(new PhyrexianMiteToken(), GetXValue.instance)"])];

/// The Mites, then with X 5 or more each creature but the tokens just made is destroyed, in battlefield order
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let before: Vec<ObjId> = game.battlefield.to_vec();
    cx.run(game, d, cx.effects);
    if d.stopped() {
        return None;
    }
    if cx.item.x < 5 {
        return Some(());
    }
    let creatures: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| before.contains(o) && game.has_type(*o, CardType::Creature))
        .collect();
    for o in creatures {
        if game.obj(o).zone == Zone::Battlefield {
            destroy(game, o);
        }
    }
    Some(())
}
