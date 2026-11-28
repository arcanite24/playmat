//! Oath of Liliana: "When Oath of Liliana enters, each opponent sacrifices a creature. At the beginning of each end
//! step, if a planeswalker entered the battlefield under your control this turn, create a 2/2 black Zombie creature
//! token."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("OathOfLiliana.OathOfLilianaCondition", super::oath_of_chandra::planeswalker_entered)];
