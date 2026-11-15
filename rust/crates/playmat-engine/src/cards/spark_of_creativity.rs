//! Spark of Creativity: "Choose target creature. Exile the top card of your library. You may have Spark of
//! Creativity deal damage to that creature equal to the exiled card's mana value. If you don't, you may play that
//! card until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SparkOfCreativity.SparkOfCreativityEffect", apply)];

/// The top card exiled; with the target still there, "Let … deal N damage to …?"; else (or declined) the card
/// playable by you until end of turn
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(&card) = game.player(you).library.first() else { return Some(()) };
    game.move_to(card, Zone::Exile);
    if let Some(creature) = cx.obj(0).filter(|o| game.obj(*o).zone == Zone::Battlefield) {
        let n = game.obj(card).def.mana_value() as i32;
        let (source, target) = (log::object(game, cx.source_id()), log::object(game, creature));
        if choose_use(game, d, you, &format!("Let {source} deal {n} damage to {target}?"))? {
            deal_damage(game, cx.source_id(), Target::Obj(creature), n, false);
            return Some(());
        }
    }
    if game.obj(card).zone == Zone::Exile {
        let turn = game.turn;
        let o = game.obj_mut(card);
        o.may_play = Some((you, turn));
        o.may_play_end_step = false;
        o.may_play_while = None;
    }
    Some(())
}
