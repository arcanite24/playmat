//! Clackbridge Troll: "Trample, haste. When Clackbridge Troll enters, target opponent creates three 0/1 white Goat
//! creature tokens. At the beginning of combat on your turn, any opponent may sacrifice a creature. If a player does,
//! tap Clackbridge Troll, you gain 3 life, and you draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ClackbridgeTroll.ClackbridgeTrollEffect", apply)
    .effects(&["new GainLifeEffect(3)", "new DrawCardSourceControllerEffect(1)"])];

/// Each opponent with a creature, in turn order: "Sacrifice a creature?", then which ("creature to sacrifice", not
/// targeted); if any was sacrificed, the Troll (if it's still that permanent) taps, then the life and the card
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut any = false;
    for k in 1..game.seats() {
        let p = (you + k) % game.seats();
        if !Game::whose("opponent", you, p) {
            continue;
        }
        let creatures: Vec<ObjId> = game
            .battlefield
            .iter()
            .copied()
            .filter(|o| game.obj(*o).controller == p && game.has_type(*o, CardType::Creature))
            .collect();
        if creatures.is_empty() || !choose_use(game, d, p, "Sacrifice a creature?")? {
            continue;
        }
        let chosen = choose_cards(game, d, p, &creatures, &creatures, 1, 1, "creature to sacrifice")?;
        let Some(&c) = chosen.first() else { continue };
        if game.obj(c).zone != Zone::Battlefield {
            continue;
        }
        sacrifice(game, c);
        if game.obj(c).zone != Zone::Battlefield {
            any = true;
        }
    }
    if any {
        if cx.source_on_battlefield(game) {
            tap(game, cx.source_id());
        }
        cx.run(game, d, &cx.effects[..2]);
    }
    (!d.stopped()).then_some(())
}
