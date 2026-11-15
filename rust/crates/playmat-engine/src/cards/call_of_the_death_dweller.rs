//! Call of the Death-Dweller: "Return up to two target creature cards with total mana value 3 or less from your
//! graveyard to the battlefield. Put a deathtouch counter on either of them. Then put a menace counter on either of
//! them."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::target(
        "CallOfTheDeathDweller.CallOfTheDeathDwellerTarget",
        "new TargetCardInYourGraveyard(0, 2, StaticFilters.FILTER_CARD_CREATURE, false)",
        can_target,
    ),
    Handler::effect("CallOfTheDeathDweller.CallOfTheDeathDwellerEffect", apply),
];

fn can_target(game: &Game, _you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    super::joshua_phoenixs_dominant::within_total_mana_value(game, id, 3)
}

/// The targets go to the battlefield together; then, of those that arrived, one chosen gets a deathtouch counter and
/// then one chosen a menace counter; only one arrived: both go on it
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let cards = cx.objs(0);
    if cards.is_empty() {
        return Some(());
    }
    let back =
        EffectDef::ReturnFromGraveyardToBattlefieldTargetEffect { tapped: false, owner: false, counters: None, attacking: false };
    cx.run(game, d, &[back]);
    if d.stopped() {
        return None;
    }
    let arrived: Vec<ObjId> = cards.iter().copied().filter(|c| game.obj(*c).zone == Zone::Battlefield).collect();
    match arrived.len() {
        0 => {}
        1 => {
            add_counters_by(game, arrived[0], "deathtouch", 1, Some(you));
            add_counters_by(game, arrived[0], "menace", 1, Some(you));
        }
        _ => {
            // (`player.choose(outcome, TargetPermanent(0, 1, filter, true), …)`: one `HumanPlayer.choose` each)
            for name in ["deathtouch", "menace"] {
                let chosen = choose_cards_rounds(game, d, you, &arrived, 0, 1)?;
                if let Some(c) = chosen.first() {
                    add_counters_by(game, *c, name, 1, Some(you));
                }
            }
        }
    }
    Some(())
}
