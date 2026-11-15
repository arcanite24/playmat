//! Kaya, Orzhov Usurper: "+1: Exile up to two target cards from a single graveyard. You gain 2 life if at least one
//! creature card was exiled this way. −1: Exile target nonland permanent with mana value 1 or less. −5: Kaya deals
//! damage to target player equal to the number of cards that player owns in exile and you gain that much life."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("KayaOrzhovUsurper.KayaOrzhovUsurperExileEffect", exile),
    Handler::effect("KayaOrzhovUsurper.KayaOrzhovUsurperDamageEffect", damage),
];

/// The targets go to exile together, then 2 life once if any of them is a creature card
fn exile(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let cards: Vec<ObjId> = cx.legal.iter().flatten().filter_map(|t| if let Target::Obj(o) = t { Some(*o) } else { None }).collect();
    for c in &cards {
        game.move_to(*c, Zone::Exile);
    }
    if cards.iter().any(|c| game.obj(*c).def.is(CardType::Creature)) {
        gain_life(game, cx.controller(), 2);
    }
    Some(())
}

/// `player.damage(count)` from the source, then the controller gains `count`
fn damage(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = cx.player(0) else {
        return Some(());
    };
    let count = game.exile.iter().filter(|c| game.obj(**c).owner == p).count() as i32;
    if count > 0 {
        deal_damage(game, cx.source_id(), Target::Player(p), count, false);
        gain_life(game, cx.controller(), count);
    }
    Some(())
}
