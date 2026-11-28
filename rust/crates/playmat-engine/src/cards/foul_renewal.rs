//! Foul Renewal: "Return target creature card from your graveyard to your hand. Target creature gets -X/-X until end
//! of turn, where X is the toughness of the card returned this way."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("FoulRenewal.FoulRenewalEffect", apply)
    .effects(&["new BoostTargetEffect(-1, -1, Duration.EndOfTurn)"])];

/// the card's toughness read before it moves; nothing without the card
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(c) = cx.obj(0) else { return Some(()) };
    if game.obj(c).zone != Zone::Graveyard {
        return Some(());
    }
    let x = -game.power_toughness(c).1;
    game.move_to(c, Zone::Hand);
    if x != 0 {
        if let Some(o) = cx.obj(1).filter(|o| game.obj(*o).zone == Zone::Battlefield) {
            let mut e = cx.effects[0].clone();
            if let EffectDef::BoostTargetEffect { power, toughness, .. } = &mut e {
                *power = Amount::Fixed(x);
                *toughness = Amount::Fixed(x);
            }
            cx.run_on(game, d, &[e], vec![Target::Obj(o)]);
        }
    }
    (!d.stopped()).then_some(())
}
