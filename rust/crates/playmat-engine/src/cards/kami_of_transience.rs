//! Kami of Transience: "At the beginning of each end step, if an enchantment was put into your graveyard from the
//! battlefield this turn, you may return this card from your graveyard to your hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("KamiOfTransience.KamiOfTransienceCondition", condition)];

/// `KamiOfTransienceWatcher`: a permanent that was an enchantment put into a graveyard from the battlefield this turn
/// (a dies event, so not a token's), owned by you; read from the card as it is in the graveyard
fn condition(game: &Game, you: Seat, _source: ObjId) -> bool {
    game.watch.to_graveyard.iter().any(|(c, _, from_battlefield)| {
        let o = game.obj(*c);
        *from_battlefield && o.owner == you && o.def.types.contains(&CardType::Enchantment)
    })
}
