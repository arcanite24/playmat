//! Garruk, Veiled Butcher: "If a creature an opponent controls would die, exile it instead. +2: Up to one target
//! creature gets -4/-1 until your next turn. −2: Each player sacrifices a creature of their choice. If you sacrificed
//! a creature this way, create a 4/4 green Beast creature token with trample. −3: Each opponent discards two cards.
//! For each opponent who didn't discard two nonland cards this way, you draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("GarrukVeiledButcher.GarrukVeiledButcherSacrificeEffect", sacrifice_mode).effects(&[
        "new SacrificeAllEffect(StaticFilters.FILTER_PERMANENT_CREATURE)",
        "new CreateTokenEffect(new Beast44TrampleToken())",
    ]),
    Handler::effect("GarrukVeiledButcher.GarrukVeiledButcherDiscardEffect", discard_mode),
];

/// `SacrificeAllEffect`, then the Beast if a creature you controlled went (each player's sacrifice is the effect's
/// only way off the battlefield here)
fn sacrifice_mode(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let yours: Vec<(ObjId, u32)> = game
        .battlefield
        .iter()
        .filter(|o| game.obj(**o).controller == you && game.has_type(**o, CardType::Creature))
        .map(|o| (*o, game.obj(*o).zcc))
        .collect();
    cx.run(game, d, &cx.effects[..1]);
    if d.stopped() {
        return None;
    }
    let sacrificed = yours.iter().any(|(o, zcc)| game.obj(*o).zone != Zone::Battlefield || game.obj(*o).zcc != *zcc);
    if sacrificed {
        cx.run_on(game, d, &cx.effects[1..2], Vec::new());
        if d.stopped() {
            return None;
        }
    }
    Some(())
}

/// Each opponent in turn order from you discards two; one who discarded fewer than two nonland cards gives you a draw
fn discard_mode(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut draws = 0;
    for i in 1..game.seats() {
        let p = (you + i) % game.seats();
        if game.player(p).lost {
            continue;
        }
        let discarded = discard(game, d, p, 2);
        if d.stopped() {
            return None;
        }
        if discarded.iter().filter(|c| !game.has_type(**c, CardType::Land)).count() < 2 {
            draws += 1;
        }
    }
    if draws > 0 {
        draw(game, you, draws);
    }
    Some(())
}
