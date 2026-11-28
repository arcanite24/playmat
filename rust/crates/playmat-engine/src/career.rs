//! Career twists (docs/rust/GameHost.md, "Career tables"): a side's edge (Java's `CareerSetup.edge`), an emblem
//! with a card's abilities (`EmblemOfCard`), and the timed twists that come in during a game
//! (`CareerTimedTwists`): triggered abilities of an "Emblem Twist" in the command zone, each once each game.
//!
//! The host makes them from the setup Java sends (`CareerSetup.hostSetup`), with the rule text and log lines Java
//! writes, so the engine only plays them.

use crate::card::{AbilityDef, CardDef, EffectDef, TokenDef, Trigger};
use crate::decide::Decider;
use crate::log;
use crate::rules;
use crate::state::{Game, ObjId, Seat, StackItem, Zone};
use std::sync::Arc;

/// A side's edge: its life set, then a life bonus, extra cards, permanents and an emblem, and its log line
/// (`CareerSetup.edge`)
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Edge {
    pub life: Option<i32>,
    pub life_bonus: i32,
    pub cards: u32,
    /// each permanent's card, and whether it enters tapped
    pub permanents: Vec<(Arc<CardDef>, bool)>,
    /// the card whose abilities the side gets as an emblem
    pub emblem: Option<Arc<CardDef>>,
    /// the log line after the player's name (" gets an edge: 2 extra cards.")
    pub log: Option<String>,
}

/// What a timed twist does as it resolves (`CareerTimedTwists.TwistEffect`): its message in the log, its edge, then
/// its life (gained, or lost when negative)
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Twist {
    pub message: Option<String>,
    pub then: Option<Edge>,
    pub gain_life: i32,
}

/// A timed twist as the host makes it: what it triggers on, its rule (Java's `getRule()`) and what it does
#[derive(Clone, Debug, PartialEq)]
pub struct Timed {
    pub on: Trigger,
    pub rule: String,
    pub twist: Arc<Twist>,
}

/// `CareerSetup.edge`: `seat`'s life set, its life bonus (never below 1), the extra cards drawn, the permanents put
/// onto the battlefield, the emblem, then the log line
pub fn edge(game: &mut Game, d: &mut dyn Decider, seat: Seat, edge: &Edge) {
    if let Some(life) = edge.life {
        game.player_mut(seat).life = life;
    }
    if edge.life_bonus != 0 {
        let p = game.player_mut(seat);
        p.life = (p.life + edge.life_bonus).max(1);
    }
    if edge.cards > 0 {
        rules::draw(game, seat, edge.cards as usize);
    }
    for (def, tapped) in &edge.permanents {
        let id = game.create_obj(def.clone(), seat, Zone::Exile);
        rules::put_onto_battlefield(game, d, id, seat);
        if *tapped && game.battlefield.contains(&id) {
            game.obj_mut(id).tapped = true;
        }
    }
    if let Some(card) = &edge.emblem {
        emblem_of_card(game, seat, card);
    }
    if let Some(line) = &edge.log {
        log::inform(game, format!("{}{line}", log::player(seat)));
    }
}

/// `EmblemOfCard`: an emblem named for the card, with its static abilities (the battlefield ones Java copies to the
/// command zone; the Career's emblems are anthems)
pub fn emblem_of_card(game: &mut Game, seat: Seat, card: &CardDef) -> ObjId {
    let abilities: Vec<AbilityDef> =
        card.abilities.iter().filter(|a| matches!(a, AbilityDef::SimpleStaticAbility { .. })).cloned().collect();
    let def = CardDef {
        set: card.set.clone(),
        ..TokenDef { name: card.name.clone(), abilities, ..Default::default() }.card()
    };
    let id = command_object(game, seat, def);
    game.obj_mut(id).card_emblem = true;
    id
}

/// `CareerTimedTwists.add`: an "Emblem Twist" holding the timed twists, as triggered abilities; none without twists
pub fn twists_emblem(game: &mut Game, seat: Seat, twists: &[Timed]) -> Option<ObjId> {
    if twists.is_empty() {
        return None;
    }
    let abilities = twists
        .iter()
        .map(|t| {
            let mut a: AbilityDef = serde_json::from_value(serde_json::json!({
                "class": "TriggeredAbility", "on": {"e": "enters"}, "effects": [], "rule": t.rule}))
            .expect("a triggered ability");
            if let AbilityDef::TriggeredAbility { on, effects, .. } = &mut a {
                *on = t.on.clone();
                *effects = vec![EffectDef::CareerTwistEffect { twist: t.twist.clone() }];
            }
            a
        })
        .collect();
    let def = TokenDef { name: "Emblem Twist".into(), abilities, ..Default::default() }.card();
    Some(command_object(game, seat, def))
}

/// An emblem `seat` gets (`Game.addEmblem`), registered as it's made, after every card's abilities
fn command_object(game: &mut Game, seat: Seat, def: CardDef) -> ObjId {
    let id = game.create_obj(Arc::new(def), seat, Zone::Library);
    game.obj_mut(id).trigger_order = (1 << 32) + id.0 as u64;
    game.move_to(id, Zone::Command);
    id
}

/// `TwistEffect.apply`: the message, the edge (" gets an edge"), then the life, gained or lost (`loseLife`, not damage)
pub fn resolve(game: &mut Game, d: &mut dyn Decider, controller: Seat, twist: &Twist) {
    if let Some(m) = &twist.message {
        log::inform(game, m.clone());
    }
    if let Some(then) = &twist.then {
        edge(game, d, controller, then);
    }
    if twist.gain_life > 0 {
        rules::gain_life(game, controller, twist.gain_life);
    } else if twist.gain_life < 0 {
        rules::lose_life(game, controller, -twist.gain_life);
    }
}

/// Whether a trigger is a Career timed twist's, which triggers once each game
pub fn is_timed(t: &Trigger) -> bool {
    matches!(t, Trigger::CareerTurn { .. } | Trigger::CareerLife { .. } | Trigger::CareerCast { .. })
}

/// `StateTriggeredAbility` for the life twists: in the order the emblem has them, the first whose player is in the
/// game at or below its life triggers, unless one of the emblem's is already waiting or on the stack (Java keeps
/// that flag by source, not by ability); one that has triggered once doesn't again
pub fn life_triggers(game: &mut Game) {
    if game.command.is_empty() {
        return;
    }
    let sources: Vec<ObjId> = game.command.to_vec();
    for src in sources {
        let held = game.pending.iter().chain(game.stack.iter()).any(|t| {
            t.card == src && t.ability.is_some() && matches!(game.trigger_of(t), Some(Trigger::CareerLife { .. }))
        });
        if held {
            continue;
        }
        let o = game.obj(src);
        let fired = o.def.abilities.iter().enumerate().find(|(i, a)| match a.trigger() {
            Some(Trigger::CareerLife { seat, life }) => {
                let p = game.player(*seat);
                !game.career_fired.contains(&(src, *i)) && !p.lost && !p.won && p.life <= *life
            }
            _ => false,
        });
        if let Some((i, _)) = fired {
            let t = StackItem { ability: Some((i, o.zcc)), ..StackItem::spell(src, o.controller) };
            game.pending.push(t);
        }
    }
}

/// The timed twists among triggers about to go on the stack that have triggered before are gone; the others count
/// (`TriggeredAbilityImpl.setTriggersLimitEachGame(1)`)
pub fn once_each_game(game: &mut Game, pending: &mut Vec<StackItem>) {
    pending.retain(|t| {
        let Some((i, _)) = t.ability else { return true };
        if !game.trigger_of(t).is_some_and(is_timed) {
            return true;
        }
        if game.career_fired.contains(&(t.card, i)) {
            return false;
        }
        game.career_fired.push((t.card, i));
        true
    });
}
