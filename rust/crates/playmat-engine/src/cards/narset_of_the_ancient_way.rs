//! Narset of the Ancient Way: "+1: You gain 2 life. Add {U}, {R}, or {W}. Spend this mana only to cast a noncreature
//! spell. −2: Draw a card, then you may discard a card. When you discard a nonland card this way, Narset deals damage
//! equal to that card's mana value to target creature or planeswalker. −6: You get an emblem with …"

use super::*;
use crate::card::{AbilityDef, Color, Mana};

pub const HANDLERS: &[Handler] = &[
    Handler::effect("NarsetOfTheAncientWay.NarsetOfTheAncientWayManaEffect", mana),
    Handler::effect("NarsetOfTheAncientWay.NarsetOfTheAncientWayDrawEffect", draw_discard),
];

/// 2 life, then `ChoiceColor` without Green and Black ("Choose color", asked even when one is owed) and that one
/// mana into the pool's conditional mana, spent only on noncreature spells
fn mana(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    gain_life(game, you, 2);
    let names = ["White", "Blue", "Red"];
    let colors = [Color::W, Color::U, Color::R];
    let q = Query::Choice { message: "Choose color".into(), choices: names.iter().map(|n| n.to_string()).collect() };
    let color = loop {
        match ask(game, d, you, &q) {
            Answer::Text(t) => {
                if let Some(i) = names.iter().position(|n| *n == t) {
                    break colors[i];
                }
            }
            Answer::Stop => return None,
            _ => {}
        }
        if d.stopped() {
            return None;
        }
    };
    let ability: AbilityDef = serde_json::from_value(serde_json::json!({
        "class": "ConditionalManaAbility",
        "filter": [{"p": "not", "x": {"p": "type", "t": "CREATURE"}}],
    }))
    .ok()?;
    let k = game.effect_mana.len();
    Arc::make_mut(&mut game.effect_mana).push(ability);
    game.player_mut(you).conditional_pool.push((Mana::of(color, 1), cx.source_id(), EFFECT_MANA + k));
    Some(())
}

/// A card drawn; a hand and "Discard a card?", then one discarded (`discardOne`); a nonland one, a reflexive "deals
/// damage equal to its mana value to target creature or planeswalker"
fn draw_discard(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    draw(game, you, 1);
    if game.player(you).hand.is_empty() || !choose_use(game, d, you, "Discard a card?")? {
        return Some(());
    }
    let discarded = discard(game, d, you, 1);
    if d.stopped() {
        return None;
    }
    let Some(&card) = discarded.first() else { return Some(()) };
    if game.obj(card).is(CardType::Land) {
        return Some(());
    }
    let x = game.obj(card).def.mana_value() as i32;
    let target: TargetDef = serde_json::from_value(serde_json::json!({
        "class": "TargetPermanent",
        "filter": [{"p": "or", "xs": [{"p": "type", "t": "CREATURE"}, {"p": "type", "t": "PLANESWALKER"}]}],
    }))
    .ok()?;
    let delayed = Delayed {
        rule: "{this} deals damage to target creature or planeswalker equal to the discarded card's mana value."
            .into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![EffectDef::DamageTargetEffect { amount: Amount::Fixed(x) }],
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: vec![target],
        condition: None,
    };
    game.pending.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), you) });
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// the conditional mana's ability and the reflexive trigger's target parse
    #[test]
    fn narset_parts_parse() {
        let a: Result<AbilityDef, _> = serde_json::from_value(serde_json::json!({
            "class": "ConditionalManaAbility",
            "filter": [{"p": "not", "x": {"p": "type", "t": "CREATURE"}}],
        }));
        assert!(a.is_ok(), "{a:?}");
        let t: Result<TargetDef, _> = serde_json::from_value(serde_json::json!({
            "class": "TargetPermanent",
            "filter": [{"p": "or", "xs": [{"p": "type", "t": "CREATURE"}, {"p": "type", "t": "PLANESWALKER"}]}],
        }));
        assert!(t.is_ok(), "{t:?}");
    }
}
