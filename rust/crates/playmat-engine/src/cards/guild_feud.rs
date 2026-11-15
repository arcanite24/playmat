//! Guild Feud: "At the beginning of your upkeep, target opponent reveals the top three cards of their library, may
//! put a creature card from among them onto the battlefield, then puts the rest into their graveyard. You do the
//! same with the top three cards of your library. If two creatures are put onto the battlefield this way, those
//! creatures fight each other."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("GuildFeud.GuildFeudEffect", apply)];

/// The opponent, then you: the top three revealed; "Put a creature card among them to the battlefield?", then one
/// creature card among them (`TargetCard(Zone.LIBRARY, …)`, chosen with `choose`) onto the battlefield; the rest into
/// the graveyard. Two creatures put this way fight (the opponent's first), as `Permanent.fight` does.
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let opponent = cx.player(0)?;
    let mut put: [Option<ObjId>; 2] = [None, None];
    for (k, player) in [opponent, you].into_iter().enumerate() {
        let mut top: Vec<ObjId> = game.player(player).library.iter().take(3).copied().collect();
        if top.is_empty() {
            continue;
        }
        if choose_use(game, d, player, "Put a creature card among them to the battlefield?")? {
            let possible: Vec<ObjId> = top.iter().copied().filter(|c| game.has_type(*c, CardType::Creature)).collect();
            if !possible.is_empty() {
                let rule = item_rule(game, cx.item);
                let chosen = choose_cards(game, d, player, &top, &possible, 1, 1, &rule)?;
                if let Some(c) = chosen.first().copied() {
                    top.retain(|t| *t != c);
                    if !put_cards(game, d, player, &[c], crate::card::PutCards::Battlefield) {
                        return None;
                    }
                    if game.obj(c).zone == Zone::Battlefield {
                        put[k] = Some(c);
                    }
                }
            }
        }
        if !put_cards(game, d, player, &top, crate::card::PutCards::Graveyard) {
            return None;
        }
    }
    // `opponentCreature.fight(controllerCreature)`, both still on the battlefield (`game.getPermanent` as each entered)
    if let [Some(a), Some(b)] = put {
        let creature = |g: &Game, o: ObjId| g.obj(o).zone == Zone::Battlefield && g.has_type(o, CardType::Creature);
        if creature(game, a) && creature(game, b) {
            let pb = game.power_toughness(b).0;
            let pa = game.power_toughness(a).0;
            deal_damage(game, b, Target::Obj(a), pb, false);
            let pa = pa.min(fight_lethal(game, b, a));
            deal_damage(game, a, Target::Obj(b), pa, false);
        }
    }
    Some(())
}
