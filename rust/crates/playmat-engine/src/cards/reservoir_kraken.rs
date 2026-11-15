//! Reservoir Kraken: "Trample, ward {2}. At the beginning of each combat, if Reservoir Kraken is untapped, any
//! opponent may tap an untapped creature they control. If they do, tap Reservoir Kraken and create a 1/1 blue Fish
//! creature token with "This token can't be blocked.""

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ReservoirKraken.ReservoirKrakenEffect", apply)
    .effects(&["new CreateTokenEffect(new FishToken())"])];

/// Each opponent with an untapped creature: "Tap an untapped creature you control?", then one chosen
/// (`chooseTarget`, not targeted) and tapped; once one was, the Kraken taps (while it's still the permanent it was)
/// and you create the Fish
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let n = game.seats();
    let mut tapped = false;
    for p in (1..n).map(|i| (you + i) % n) {
        if game.player(p).lost {
            continue;
        }
        let untapped: Vec<ObjId> = game
            .battlefield
            .iter()
            .copied()
            .filter(|o| game.obj(*o).controller == p && game.has_type(*o, CardType::Creature) && !game.obj(*o).tapped)
            .collect();
        if untapped.is_empty() || !choose_use(game, d, p, "Tap an untapped creature you control?")? {
            continue;
        }
        let rule = item_rule(game, cx.item);
        if let Some(o) = choose_cards(game, d, p, &untapped, &untapped, 1, 1, &rule)?.first().copied() {
            if game.obj(o).zone == Zone::Battlefield && !game.obj(o).tapped {
                tap(game, o);
                tapped = true;
            }
        }
    }
    if tapped {
        if cx.source_on_battlefield(game) {
            tap(game, cx.source_id());
        }
        cx.run_on(game, d, &cx.effects[..1], Vec::new());
    }
    Some(())
}
