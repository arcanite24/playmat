//! Braids, Arisen Nightmare: "At the beginning of your end step, you may sacrifice an artifact, creature,
//! enchantment, land, or planeswalker. If you do, each opponent may sacrifice a permanent of their choice that shares
//! a card type with it. For each opponent who doesn't, that player loses 2 life and you draw a card."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("BraidsArisenNightmare.BraidsArisenNightmareEffect", apply).filters(&["filter"])];

/// `CardType.toString`, lowercased, as `SharesCardTypePredicate.toString` writes it
fn type_name(t: CardType) -> &'static str {
    match t {
        CardType::Artifact => "artifact",
        CardType::Battle => "battle",
        CardType::Creature => "creature",
        CardType::Enchantment => "enchantment",
        CardType::Instant => "instant",
        CardType::Kindred => "kindred",
        CardType::Land => "land",
        CardType::Planeswalker => "planeswalker",
        CardType::Sorcery => "sorcery",
    }
}

/// `SharesCardTypePredicate.toString`: "a", "a or b", "a, b, or c"
fn types_text(types: &[CardType]) -> String {
    let names: Vec<&str> = types.iter().map(|t| type_name(*t)).collect();
    match names.len() {
        0 => String::new(),
        1 => names[0].to_string(),
        2 => format!("{} or {}", names[0], names[1]),
        n => format!("{}, or {}", names[..n - 1].join(", "), names[n - 1]),
    }
}

/// `CardUtil.addArticle`
fn with_article(s: &str) -> String {
    if s.starts_with(['a', 'e', 'i', 'o', 'u', 'A', 'E', 'I', 'O', 'U']) {
        format!("an {s}")
    } else {
        format!("a {s}")
    }
}

/// `TargetSacrifice` of one: a PICK_TARGET among `possible` (picked for the player when only one fits)
fn choose_one(game: &mut Game, d: &mut dyn Decider, seat: Seat, possible: &[ObjId]) -> Option<ObjId> {
    choose_cards(game, d, seat, possible, possible, 1, 1, "sacrifice").map(|c| c.first().copied()).unwrap_or(None)
}

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mine: Vec<ObjId> = matching(game, &cx.filters[0], you, cx.source_id())
        .into_iter()
        .filter(|o| game.obj(*o).controller == you)
        .collect();
    if mine.is_empty() {
        return Some(());
    }
    let Some(chosen) = choose_one(game, d, you, &mine) else {
        return if d.stopped() { None } else { Some(()) };
    };
    // its card types as it was chosen (the predicate is made before the sacrifice)
    let mut types: Vec<CardType> = game.obj(chosen).def.types.clone();
    types.retain(|t| game.has_type(chosen, *t));
    let text = types_text(&types);
    if game.obj(chosen).zone != Zone::Battlefield {
        return Some(());
    }
    sacrifice(game, chosen);
    if game.obj(chosen).zone == Zone::Battlefield {
        return Some(());
    }
    for i in 1..game.seats() {
        let opp = (you + i) % game.seats();
        if game.player(opp).lost {
            continue;
        }
        let shares: Vec<ObjId> = game
            .battlefield
            .iter()
            .copied()
            .filter(|o| game.obj(*o).controller == opp && types.iter().any(|t| game.has_type(*o, *t)))
            .collect();
        let mut sacrificed = false;
        if !shares.is_empty() {
            let message = format!("Sacrifice {}?", with_article(&text));
            if choose_use(game, d, opp, &message)? {
                if let Some(p) = choose_one(game, d, opp, &shares) {
                    if game.obj(p).zone == Zone::Battlefield {
                        sacrifice(game, p);
                        sacrificed = game.obj(p).zone != Zone::Battlefield;
                    }
                } else if d.stopped() {
                    return None;
                }
            }
        }
        if !sacrificed {
            lose_life(game, opp, 2);
            draw(game, you, 1);
        }
    }
    Some(())
}
