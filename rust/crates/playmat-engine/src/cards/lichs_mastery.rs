//! Lich's Mastery: "You can't lose the game. … Whenever you lose life, for each 1 life you lost, exile a permanent
//! you control or a card from your hand or graveyard."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::rule("LichsMastery.LichsMasteryCantLoseEffect", Rule::CantLoseOrWin(you_lose)),
    Handler::effect("LichsMastery.LichsMasteryLoseLifeEffect", exile_for_each),
];

/// `LOSES` for the source's controller
fn you_lose(_: &Game, cx: &StaticCx, seat: Seat, wins: bool) -> bool {
    !wins && seat == cx.controller
}

/// For each 1 life lost (the trigger's X): a permanent you control (asked when there are also cards in your hand or
/// graveyard), else a card from your hand (asked when your graveyard has cards too), else one from your graveyard
fn exile_for_each(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    // (`tryToAutoChoose` reads the ability's "exile": a card or permanent of yours is never picked by itself)
    let rule = item_rule(game, cx.item);
    for _ in 0..cx.item.x.max(0) {
        let hand = game.player(you).hand.clone();
        let graveyard = game.player(you).graveyard.clone();
        let permanents: Vec<ObjId> =
            game.battlefield.iter().copied().filter(|o| game.obj(*o).controller == you).collect();
        let from_battlefield = hand.len() + graveyard.len() == 0
            || (!permanents.is_empty()
                && choose_use(game, d, you, "Exile permanent you control? (No = from hand or graveyard)")?);
        if from_battlefield {
            // `controller.choose(TargetControlledPermanent(1, 1, …, true))`, then `ExileTargetEffect`
            let chosen = choose_in_rounds(game, d, you, &permanents, 1, 1, &rule)?;
            if let Some(o) = chosen.first() {
                game.move_to(*o, Zone::Exile);
            }
        } else if graveyard.is_empty()
            || (!hand.is_empty() && choose_use(game, d, you, "Exile a card from your hand? (No = from graveyard)")?)
        {
            let chosen = choose_in_rounds(game, d, you, &hand, 1, 1, &rule)?;
            if let Some(c) = chosen.first() {
                game.move_to(*c, Zone::Exile);
            }
        } else {
            // (`target.choose`: `TargetImpl.choose`)
            let chosen = choose_in_rounds(game, d, you, &graveyard, 1, 1, &rule)?;
            if let Some(c) = chosen.first() {
                game.move_to(*c, Zone::Exile);
            }
        }
    }
    Some(())
}
