//! Break Out: "Look at the top six cards of your library. You may reveal a creature card from among them. If that card
//! has mana value 2 or less, you may put it onto the battlefield and it gains haste until end of turn. If you didn't
//! put the revealed card onto the battlefield this way, put it into your hand. Put the rest on the bottom of your
//! library in a random order."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BreakOut.BreakOutEffect", apply)
    .effects(&["new GainAbilityTargetEffect(HasteAbility.getInstance(), Duration.EndOfTurn)"])];

/// `chooseTarget(TargetCard(0, 1, creature))` over the six; the pick revealed, then "Put it onto the battlefield?"
/// for mana value 2 or less (with haste), else the hand; the rest to the bottom in a random order
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut cards: Vec<ObjId> = game.player(you).library.iter().take(6).copied().collect();
    let creatures: Vec<ObjId> = cards.iter().copied().filter(|c| game.obj(*c).is(CardType::Creature)).collect();
    let rule = item_rule(game, cx.item);
    let picked = if cards.is_empty() { Vec::new() } else { choose_cards(game, d, you, &cards, &creatures, 0, 1, &rule)? };
    if let Some(&card) = picked.first() {
        let zcc = game.obj(cx.source_id()).zcc;
        game.revealed.push(crate::state::Revealed {
            source: cx.source_id(),
            zcc,
            paren: false,
            cards: vec![(card, Zone::Library)],
        });
        cards.retain(|c| *c != card);
        let small = game.obj(card).def.mana_value() <= 2;
        let onto = small && {
            let q = Query::YesNo { message: "Put it onto the battlefield?".into() };
            loop {
                match ask(game, d, you, &q) {
                    Answer::Bool(b) => break b,
                    Answer::Stop => return None,
                    _ => {}
                }
            }
        };
        if onto {
            enter_battlefield(game, d, card, you, false);
            if game.obj(card).zone == Zone::Battlefield {
                cx.run_on(game, d, &cx.effects[..1], vec![Target::Obj(card)]);
            }
        } else {
            game.move_to(card, Zone::Hand);
        }
    }
    if !put_cards(game, d, you, &cards, crate::card::PutCards::BottomRandom) {
        return None;
    }
    Some(())
}
