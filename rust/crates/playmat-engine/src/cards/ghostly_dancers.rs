//! Ghostly Dancers: "Flying. When this creature enters, return an enchantment card from your graveyard to your hand
//! or unlock a locked door of a Room you control. Eerie — Whenever an enchantment you control enters and whenever you
//! fully unlock a Room, create a 3/1 white Spirit creature token with flying."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("GhostlyDancers.GhostlyDancersEffect", apply)];

/// "Return an enchantment …?": yes, one enchantment card of your graveyard chosen (not targeted) goes to your hand;
/// no, a Room you control chosen, and a locked door of it unlocked (which one asked when both are locked)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let message = "Return an enchantment card from your graveyard to your hand or unlock a locked door of a Room you \
                   control?";
    let rule = item_rule(game, cx.item);
    if choose_use(game, d, you, message)? {
        let yard = game.player(you).graveyard.clone();
        let possible: Vec<ObjId> = yard.iter().copied().filter(|c| game.obj(*c).is(CardType::Enchantment)).collect();
        if possible.is_empty() {
            return Some(());
        }
        if let Some(&card) = choose_cards(game, d, you, &possible, &possible, 1, 1, &rule)?.first() {
            if game.obj(card).zone == Zone::Graveyard {
                game.move_to(card, Zone::Hand);
            }
        }
        return Some(());
    }
    let rooms: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == you && game.has_subtype(*o, "Room"))
        .collect();
    if rooms.is_empty() {
        return Some(());
    }
    let Some(&room) = choose_cards(game, d, you, &rooms, &rooms, 1, 1, &rule)?.first() else { return Some(()) };
    if game.obj(room).zone != Zone::Battlefield {
        return Some(());
    }
    let doors = game.obj(room).doors;
    let left = match doors {
        [true, true] => return Some(()),
        [false, true] => true,
        [true, false] => false,
        [false, false] => choose_use(game, d, you, "Unlock the left door or the right door?")?,
    };
    unlock_door(game, room, if left { 0 } else { 1 });
    Some(())
}
