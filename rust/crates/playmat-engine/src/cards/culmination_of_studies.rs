//! Culmination of Studies: "Exile the top X cards of your library. For each land card exiled this way, create a
//! Treasure token. For each blue card exiled this way, draw a card. For each red card exiled this way, Culmination of
//! Studies deals 1 damage to each opponent."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("CulminationOfStudies.CulminationOfStudiesEffect", apply)
    .effects(&["new CreateTokenEffect(new TreasureToken())"])];

/// the top X exiled together, then the Treasures, the draws and the damage to each opponent, in that order
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let x = cx.item.x.max(0) as usize;
    let top: Vec<ObjId> = game.player(you).library.iter().take(x).copied().collect();
    for c in &top {
        game.move_to(*c, Zone::Exile);
    }
    let exiled: Vec<ObjId> = top.into_iter().filter(|c| game.obj(*c).zone == Zone::Exile).collect();
    let lands = exiled.iter().filter(|c| game.has_type(**c, CardType::Land)).count() as i32;
    let blue = exiled.iter().filter(|c| game.colors(**c).contains(&Color::U)).count();
    let red = exiled.iter().filter(|c| game.colors(**c).contains(&Color::R)).count() as i32;
    if lands > 0 {
        let mut e = cx.effects[0].clone();
        if let EffectDef::CreateTokenEffect { amount, .. } = &mut e {
            *amount = Amount::Fixed(lands);
        }
        cx.run(game, d, &[e]);
        if d.stopped() {
            return None;
        }
    }
    if blue > 0 {
        draw(game, you, blue);
    }
    if red > 0 {
        let n = game.seats();
        for i in 1..n {
            let p = (you + i) % n;
            if !game.player(p).lost {
                deal_damage(game, cx.source_id(), Target::Player(p), red, false);
            }
        }
    }
    Some(())
}
