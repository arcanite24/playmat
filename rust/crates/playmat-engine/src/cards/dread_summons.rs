//! Dread Summons: "Each player mills X cards. For each creature card put into a graveyard this way, you create a
//! tapped 2/2 black Zombie creature token."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DreadSummons.DreadSummonsEffect", apply)
    .effects(&["new CreateTokenEffect(new ZombieToken(), 1, true, false)"])];

/// each player from you in turn order mills X (`MillCardsEachPlayerEffect`'s way); the creature cards that reached a
/// graveyard are counted, then the tokens are made together, tapped
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let n = cx.item.x.max(0) as usize;
    let mut count = 0;
    for k in 0..game.seats() {
        let p = (you + k) % game.seats();
        let top: Vec<ObjId> = game.player(p).library.iter().take(n).copied().collect();
        note(game, EventKind::Milled, Some(p), None, None, top.len() as i32);
        for c in &top {
            game.move_to(*c, Zone::Graveyard);
        }
        count += top.iter().filter(|c| game.obj(**c).zone == Zone::Graveyard && game.has_type(**c, CardType::Creature)).count();
    }
    if count > 0 {
        let mut e = cx.effects[0].clone();
        if let EffectDef::CreateTokenEffect { amount, .. } = &mut e {
            *amount = Amount::Fixed(count as i32);
        }
        cx.run_on(game, d, &[e], Vec::new());
    }
    Some(())
}
