//! Klothys, God of Destiny: "Indestructible. As long as your devotion to red and green is less than seven, Klothys
//! isn't a creature. At the beginning of your first main phase, exile target card from a graveyard. If it was a land
//! card, add {R} or {G}. Otherwise, you gain 2 life and Klothys deals 2 damage to each opponent."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("KlothysGodOfDestiny.KlothysGodOfDestinyEffect", apply)];

/// the target exiled; a land: `chooseUse("Choose a color of mana to add", "Red", "Green")` (yes is red); else 2 life
/// and 2 damage to each opponent
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(card) = cx.obj(0) else { return Some(()) };
    let land = game.obj(card).is(CardType::Land);
    game.move_to(card, Zone::Exile);
    if land {
        let red = choose_use(game, d, you, "Choose a color of mana to add")?;
        let k = if red { Color::R } else { Color::G }.index();
        game.player_mut(you).mana_pool.colored[k] += 1;
        return Some(());
    }
    gain_life(game, you, 2);
    for k in 1..game.seats() {
        let p = (you + k) % game.seats();
        if !game.player(p).lost {
            deal_damage(game, cx.source_id(), Target::Player(p), 2, false);
        }
    }
    Some(())
}
