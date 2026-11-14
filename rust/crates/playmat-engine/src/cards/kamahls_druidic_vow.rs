//! Kamahl's Druidic Vow: "Look at the top X cards of your library. You may put any number of land and/or legendary
//! permanent cards with mana value X or less from among them onto the battlefield. Put the rest into your graveyard."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("KamahlsDruidicVow.KamahlsDruidicVowEffect", apply)];

/// The top X looked at (not revealed); `choose(cards, TargetCard(0, any, …))`, asked even with none to pick; those
/// onto the battlefield, then the rest into the graveyard
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let x = cx.item.x.max(0);
    let cards: Vec<ObjId> = game.player(you).library.iter().take(x as usize).copied().collect();
    if cards.is_empty() {
        return Some(());
    }
    let fits = |g: &Game, c: ObjId| {
        let def = &g.obj(c).def;
        def.is_permanent_card()
            && def.mana_value() as i32 <= x
            && (def.is(CardType::Land) || def.supertypes.iter().any(|t| t == "LEGENDARY"))
    };
    let possible: Vec<ObjId> = cards.iter().copied().filter(|c| fits(game, *c)).collect();
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &cards, &possible, 0, cards.len(), &rule)?;
    let rest: Vec<ObjId> = cards.iter().copied().filter(|c| !chosen.contains(c)).collect();
    if !put_cards(game, d, you, &chosen, crate::card::PutCards::Battlefield)
        || !put_cards(game, d, you, &rest, crate::card::PutCards::Graveyard)
    {
        return None;
    }
    Some(())
}
