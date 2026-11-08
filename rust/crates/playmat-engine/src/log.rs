//! The game log (docs/rust/Views.md): the lines Java's `game.informPlayers` writes, as the engine writes them while
//! `Game::log` is kept. Players are named by seat (`A`, `B`; Java's lines are mapped to the same), objects as
//! `GameLog.getColoredObjectIdName` writes them, with their references.

use crate::prompt::{object_html, player_html};
use crate::state::{Game, ObjId, Seat, Zone};

/// a seat's name in the log
pub fn seat_name(seat: Seat) -> &'static str {
    ["A", "B", "C", "D"].get(seat).copied().unwrap_or("?")
}

/// `Player.getLogName`
pub fn player(seat: Seat) -> String {
    player_html(seat_name(seat))
}

/// `MageObject.getLogName`
pub fn object(game: &Game, id: ObjId) -> String {
    object_html(game, id)
}

/// Whether the log is kept: build a line only then
pub fn kept(game: &Game) -> bool {
    game.log.is_some()
}

/// `game.informPlayers`
pub fn inform(game: &mut Game, line: String) {
    if let Some(log) = game.log.as_mut() {
        log.push(line);
    }
}

/// `Zone.toString().toLowerCase()`
pub fn zone(z: Zone) -> &'static str {
    match z {
        Zone::Library => "library",
        Zone::Hand => "hand",
        Zone::Battlefield => "battlefield",
        Zone::Graveyard => "graveyard",
        Zone::Stack => "stack",
        Zone::Exile => "exile zone",
        Zone::Command => "command",
    }
}

/// `CardUtil.numberToText(n, "a")`
pub fn number(n: usize) -> String {
    const WORDS: [&str; 21] = [
        "zero",
        "a",
        "two",
        "three",
        "four",
        "five",
        "six",
        "seven",
        "eight",
        "nine",
        "ten",
        "eleven",
        "twelve",
        "thirteen",
        "fourteen",
        "fifteen",
        "sixteen",
        "seventeen",
        "eighteen",
        "nineteen",
        "twenty",
    ];
    WORDS.get(n).map_or_else(|| n.to_string(), |w| w.to_string())
}

/// `Target.getTargetedName` for each target with some chosen, after " targeting " (`getTargetDescriptionForLog`)
pub fn targets(game: &Game, targets: &[Vec<crate::state::Target>]) -> String {
    let named: Vec<String> = targets
        .iter()
        .filter(|t| !t.is_empty())
        .map(|t| {
            t.iter()
                .map(|x| match x {
                    crate::state::Target::Player(p) => player(*p),
                    crate::state::Target::Obj(o) => object(game, *o),
                })
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect();
    if named.is_empty() {
        String::new()
    } else {
        format!(" targeting {}", named.join(", "))
    }
}

/// `Spell.getActivatedMessage`: "casts" a spell just cast, the top of the stack
pub fn casts(game: &Game, seat: Seat, id: ObjId) -> String {
    let item = game.stack.iter().rev().find(|i| i.card == id && i.ability.is_none());
    let o = game.obj(id);
    let mut line = format!("{} casts {}", player(seat), object(game, id));
    // (flashback: cast from a graveyard with its flashback face, `Card.adventurer` the whole card)
    if o.flashback || (o.cast_from == Some(Zone::Graveyard) && o.adventurer.is_some()) {
        line += " using Flashback";
    }
    if o.cast_from == Some(Zone::Graveyard) {
        line += " from graveyard";
    }
    if let Some(item) = item {
        // `getOptionalTextSuffix`: a promised gift
        if item.gift_promised {
            for a in &o.def.abilities {
                if let crate::card::AbilityDef::GiftAbility { name, .. } = a {
                    line += &format!(" with Gift {}", name);
                }
            }
        }
        if o.def.spell.modes.len() > 1 {
            for m in &item.modes {
                line += &format!(" (mode {})", m + 1);
            }
        }
        line += &targets(game, &item.targets);
    }
    line += &format!(" from {}", zone(o.cast_from.unwrap_or(Zone::Hand)));
    line
}

/// what's resolving: the source of what happens (`CardUtil.getSourceLogName`)
pub fn source(game: &Game) -> Option<ObjId> {
    game.resolving.map(|(_, card)| card)
}

/// `TriggeredAbilityImpl.getGameLogMessage`, as `PlayerImpl.triggerAbility` writes it: a trigger put on the stack
pub fn triggers(game: &Game, seat: Seat, item: &crate::state::StackItem) -> String {
    let src = object(game, item.card);
    let rule = crate::view::item_rule(game, item).replace("{this}", &src);
    let mut line = format!("{} - Ability triggers: {} - {}", player(seat), src, rule);
    let t = targets(game, &item.targets);
    if !t.is_empty() {
        line += &format!(" - {}", t);
    }
    line
}

/// `AbilityImpl.getGameLogMessage`, as `PlayerImpl.activateAbility` writes it: its effects' text (the rule after its
/// costs), its source, its targets
pub fn activates(game: &Game, seat: Seat, item: &crate::state::StackItem) -> String {
    let src = object(game, item.card);
    let rule = crate::view::item_rule(game, item);
    // (`Modes.getText`: the effects' text, which `getRule` capitalizes after the costs; a few effects' own text
    // starts with a capital, which this can't tell)
    let text = rule.split_once(": ").map_or(rule.as_str(), |(_, t)| t);
    let mut c = text.chars();
    let text = c.next().map(|f| f.to_lowercase().chain(c).collect::<String>()).unwrap_or_default();
    let text = text.replace("{this}", &src);
    format!("{} activates: {} from {}{}", player(seat), text, src, targets(game, &item.targets))
}

/// A line or message with its objects and numbers left out, to group alike ones (`OBJ`, `N`)
pub fn template(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find("<font") {
        out.push_str(&rest[..i]);
        let Some(j) = rest[i..].find("</font>") else { break };
        out.push_str("OBJ");
        rest = &rest[i + j + 7..];
        if let Some(r) = rest.strip_prefix(" [") {
            if let Some(k) = r.find(']') {
                rest = &r[k + 1..];
            }
        }
    }
    out.push_str(rest);
    out.chars().map(|c| if c.is_ascii_digit() { 'N' } else { c }).take(100).collect()
}
