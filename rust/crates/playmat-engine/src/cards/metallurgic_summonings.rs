//! Metallurgic Summonings: "Whenever you cast an instant or sorcery spell, create an X/X colorless Construct artifact
//! creature token, where X is that spell's mana value. {3}{U}{U}, Exile this enchantment: Return all instant and
//! sorcery cards from your graveyard to your hand. Activate only if you control six or more artifacts."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("MetallurgicSummonings.MetallurgicSummoningsTokenEffect", token)
        .effects(&["new CreateTokenEffect(new MetallurgicSummoningsConstructToken())"]),
    Handler::effect("MetallurgicSummonings.MetallurgicSummoningsReturnEffect", return_all)
        .filters(&["StaticFilters.FILTER_CARDS_INSTANT_AND_SORCERY"]),
];

/// the cast trigger's X is the spell's mana value (`getValue("spellCast")`)
fn token(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let x = cx.item.x.max(0);
    let mut e = cx.effects[0].clone();
    if let EffectDef::CreateTokenEffect { token, .. } = &mut e {
        token.power = x;
        token.toughness = x;
    }
    cx.run(game, d, &[e]);
    Some(())
}

fn return_all(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let cards: Vec<ObjId> = game
        .player(you)
        .graveyard
        .iter()
        .copied()
        .filter(|c| game.filter_matches(&cx.filters[0], you, Some(cx.source_id()), *c))
        .collect();
    for c in cards {
        game.move_to(c, Zone::Hand);
    }
    Some(())
}
