//! The dungeons' own room effects (`mage.game.command.dungeons`): Tomb of Annihilation's Veils of Fear, Oubliette and
//! Sandfall Cell, Dungeon of the Mad Mage's Runestone Caverns and Mad Wizard's Lair. The rest of the rooms are
//! framework effects (the transpiler's `dungeon`).

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("TombOfAnnihilationDungeon.VeilsOfFearEffect", veils_of_fear),
    Handler::effect("TombOfAnnihilationDungeon.OublietteEffect", oubliette),
    Handler::effect("TombOfAnnihilationDungeon.SandfallCellEffect", sandfall_cell),
    Handler::effect("DungeonOfTheMadMageDungeon.RunestoneCavernsEffect", runestone_caverns),
    Handler::effect("DungeonOfTheMadMageDungeon.MadWizardsLairEffect", mad_wizards_lair),
];

/// the players from you in turn order (`getPlayersInRange`)
fn players_from(game: &Game, you: Seat) -> Vec<Seat> {
    (0..game.seats()).map(|k| (you + k) % game.seats()).filter(|p| !game.player(*p).lost).collect()
}

/// "an artifact, a creature, or a land" you control (`TombOfAnnihilationDungeon.filter`)
fn artifact_creature_or_land(game: &Game, seat: Seat) -> Vec<ObjId> {
    game.battlefield
        .iter()
        .copied()
        .filter(|b| {
            game.obj(*b).controller == seat
                && [CardType::Artifact, CardType::Creature, CardType::Land].iter().any(|t| game.has_type(*b, *t))
        })
        .collect()
}

/// Veils of Fear: "Each player loses 2 life unless they discard a card." Each player from you picks up to one card
/// (`TargetDiscard(0, 1)`, asked even with an empty hand), then each discards it or loses 2 life. Java applies them
/// in a `HashMap`'s order of the players' ids; here, in turn order.
fn veils_of_fear(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let rule = item_rule(game, cx.item);
    let mut picks: Vec<(Seat, Option<ObjId>)> = Vec::new();
    for p in players_from(game, cx.controller()) {
        let hand = game.player(p).hand.clone();
        let chosen = choose_cards(game, d, p, &hand, &hand, 0, 1, &rule)?;
        picks.push((p, chosen.first().copied()));
    }
    for (p, c) in picks {
        match c.filter(|c| game.obj(*c).zone == Zone::Hand) {
            Some(c) => discard_card(game, p, c),
            None => lose_life(game, p, 2),
        }
    }
    Some(())
}

/// Oubliette: "Discard a card and sacrifice an artifact, a creature, and a land." The discard, then up to one of
/// each (`OublietteTarget`: as many as the types can be shared out, `TargetSacrifice` with that many, a permanent
/// offered only while the chosen ones can each take a different type), sacrificed together.
fn oubliette(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    discard(game, d, you, 1);
    if d.stopped() {
        return None;
    }
    let all = artifact_creature_or_land(game, you);
    let n = role_count(game, &all).min(3);
    if n == 0 {
        return Some(());
    }
    let rule = item_rule(game, cx.item).to_lowercase();
    let mut chosen: Vec<ObjId> = Vec::new();
    loop {
        if chosen.len() >= n {
            break;
        }
        // `possibleTargets`: the ones that keep a type for each chosen permanent
        let possible: Vec<ObjId> = all
            .iter()
            .copied()
            .filter(|c| !chosen.contains(c))
            .filter(|c| {
                let mut with: Vec<ObjId> = chosen.clone();
                with.push(*c);
                role_count(game, &with) == with.len()
            })
            .collect();
        if possible.is_empty() {
            break;
        }
        // `tryToAutoChoose`: exactly the ones still needed, and none of them yours to feel bad about ("sacrifice")
        if possible.len() == n - chosen.len() && !rule.contains("search") {
            let auto = game.player(you).auto_target_all;
            if let Some(c) = possible.iter().copied().find(|c| auto || game.obj(*c).controller != you) {
                chosen.push(c);
                continue;
            }
        }
        match ask(game, d, you, &Query::Cards { objs: possible.clone(), required: true }) {
            Answer::Obj(o) if chosen.contains(&o) => chosen.retain(|c| *c != o),
            Answer::Obj(o) if possible.contains(&o) => chosen.push(o),
            Answer::Stop => return None,
            _ => {}
        }
        if d.stopped() {
            return None;
        }
    }
    for c in chosen {
        if game.obj(c).zone == Zone::Battlefield {
            sacrifice(game, c);
        }
    }
    Some(())
}

/// `CardTypeAssignment.getRoleCount` over artifact, creature and land: how many of these permanents can each take a
/// different one of the three types
fn role_count(game: &Game, objs: &[ObjId]) -> usize {
    let types = [CardType::Artifact, CardType::Creature, CardType::Land];
    let roles: Vec<Vec<usize>> =
        objs.iter().map(|o| (0..3).filter(|k| game.has_type(*o, types[*k])).collect()).collect();
    // at most three roles: try every assignment
    fn best(roles: &[Vec<usize>], used: [bool; 3]) -> usize {
        let Some((first, rest)) = roles.split_first() else { return 0 };
        let mut most = best(rest, used);
        for r in first {
            if !used[*r] {
                let mut u = used;
                u[*r] = true;
                most = most.max(1 + best(rest, u));
            }
        }
        most
    }
    best(&roles, [false; 3])
}

/// Sandfall Cell: "Each player loses 2 life unless they sacrifice an artifact, a creature, or a land." Each player
/// from you picks up to one of theirs (`TargetPermanent(0, 1)`), then each sacrifices it or loses 2 life (Java's
/// `HashMap` order of the players' ids; here, turn order).
fn sandfall_cell(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let rule = item_rule(game, cx.item);
    let mut picks: Vec<(Seat, Option<(ObjId, u32)>)> = Vec::new();
    for p in players_from(game, cx.controller()) {
        let possible = artifact_creature_or_land(game, p);
        let chosen = choose_cards(game, d, p, &possible, &possible, 0, 1, &rule)?;
        picks.push((p, chosen.first().map(|c| (*c, game.obj(*c).zcc))));
    }
    for (p, c) in picks {
        match c.filter(|(c, zcc)| game.obj(*c).zone == Zone::Battlefield && game.obj(*c).zcc == *zcc) {
            Some((c, _)) => sacrifice(game, c),
            None => lose_life(game, p, 2),
        }
    }
    Some(())
}

/// Runestone Caverns: "Exile the top two cards of your library. You may play them." As Java does it, while the
/// ability resolves: "Play <card>?" for each of them, cast paying its costs (a land can't be: `chooseAbilityForCast`
/// finds no spell ability), then, with some still in exile, "Continue playing the exiled cards?"
fn runestone_caverns(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut cards: Vec<ObjId> = game.player(you).library.iter().take(2).copied().collect();
    if cards.is_empty() {
        return Some(());
    }
    for c in &cards {
        game.move_to(*c, Zone::Exile);
    }
    cards.retain(|c| game.obj(*c).zone == Zone::Exile);
    while !cards.is_empty() {
        for c in cards.clone() {
            let message = format!("Play {}?", game.obj(c).def.name);
            if !choose_use(game, d, you, &message)? {
                continue;
            }
            if game.obj(c).zone == Zone::Exile && !game.obj(c).def.is(CardType::Land) {
                cast_allowed(game, d, you, c);
                if d.stopped() {
                    return None;
                }
            }
        }
        cards.retain(|c| game.obj(*c).zone == Zone::Exile);
        if cards.is_empty() || !choose_use(game, d, you, "Continue playing the exiled cards?")? {
            break;
        }
    }
    Some(())
}

/// Mad Wizard's Lair: "Draw three cards and reveal them. You may cast one of them without paying its mana cost."
/// The top three are drawn (nothing more if a replacement drew others), revealed, then up to one nonland card among them still in
/// hand is chosen (`TargetCardInHand(0, 1)`) and cast free.
fn mad_wizards_lair(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let top: Vec<ObjId> = game.player(you).library.iter().take(3).copied().collect();
    draw(game, you, 3);
    let drawn = top.iter().filter(|c| game.obj(**c).zone == Zone::Hand).count();
    if drawn != top.len() {
        return Some(());
    }
    let zcc = game.obj(cx.source_id()).zcc;
    let shown: Vec<(ObjId, Zone)> = top.iter().map(|c| (*c, Zone::Hand)).collect();
    game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards: shown });
    let possible: Vec<ObjId> =
        top.iter().copied().filter(|c| game.obj(*c).zone == Zone::Hand && !game.obj(*c).def.is(CardType::Land)).collect();
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &top, &possible, 0, 1, &rule)?;
    if let Some(c) = chosen.first().copied().filter(|c| game.obj(*c).zone == Zone::Hand) {
        cast_allowed_with(game, d, you, c, true);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}
