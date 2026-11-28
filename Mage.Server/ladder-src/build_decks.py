#!/usr/bin/env python3
"""Generates the dedicated AI Ranked ladder deck pool under
Mage.Server/src/main/resources/ladder/decks/{standard,pioneer,modern}/ and merges their
playbooks into rust/crates/playmat-ai/src/playbooks.json and
Mage.Server/src/main/resources/ladder/playbooks.json.

Every card is looked up in rust/crates/playmat-engine/cards/views.json and checked against
rust/crates/playmat-engine/cards/verified.txt so 100% of ladder decks run natively on the Rust host.
"""
import gzip
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
SERVER = os.path.dirname(HERE)
REPO = os.path.dirname(SERVER)
VIEWS_PATH = os.path.join(REPO, "rust", "crates", "playmat-engine", "cards", "views.json.gz")
VERIFIED_PATH = os.path.join(REPO, "rust", "crates", "playmat-engine", "cards", "verified.txt")
LADDER_DECKS = os.path.join(SERVER, "src", "main", "resources", "ladder", "decks")
RUST_PLAYBOOKS = os.path.join(REPO, "rust", "crates", "playmat-ai", "src", "playbooks.json")
JAVA_PLAYBOOKS = os.path.join(SERVER, "src", "main", "resources", "ladder", "playbooks.json")

# Preferred printings for basic lands (FDN / TRK are universally Standard+Pioneer+Modern legal)
BASIC_PRINTINGS = {
    "Plains": ("FDN", "273"),
    "Island": ("FDN", "275"),
    "Swamp": ("FDN", "277"),
    "Mountain": ("FDN", "279"),
    "Forest": ("FDN", "281"),
}

# Each entry:
#   (format, slug, name, tier, archetype, profile, tagline, main_list, sb_list, playbook_extra)
# main_list / sb_list: [(count, card_name), ...]
DECKS = [
    # ===================== STANDARD (+20 -> 40 total) =====================
    (
        "standard", "tier1-mono-black-demons", "Mono-Black Demons", "meta", "midrange", "midrange",
        "Hand disruption and Unholy Annex demon beats",
        [
            (4, "Deep-Cavern Bat"), (4, "Preacher of the Schism"), (4, "Unstoppable Slasher"),
            (3, "Bloodletter of Aclazotz"), (2, "Vein Ripper"), (4, "Unholy Annex // Ritual Chamber"),
            (4, "Shoot the Sheriff"), (4, "Nowhere to Run"), (3, "Duress"), (2, "Hero's Downfall"),
            (2, "Bitter Triumph"), (2, "Soulstone Sanctuary"), (2, "Demolition Field"), (20, "Swamp"),
        ],
        [(1, "Duress"), (2, "Withering Torment"), (3, "Pilfer"), (3, "Strategic Betrayal"), (3, "Stab"), (3, "Deadly Cover-Up")],
        {
            "keyCards": ["Unholy Annex // Ritual Chamber", "Unstoppable Slasher", "Bloodletter of Aclazotz", "Deep-Cavern Bat"],
            "roles": {
                "Duress": ["early_play"], "Deep-Cavern Bat": ["early_play", "engine"],
                "Preacher of the Schism": ["engine", "payoff"], "Unholy Annex // Ritual Chamber": ["engine", "payoff"],
                "Unstoppable Slasher": ["payoff", "finisher"], "Bloodletter of Aclazotz": ["finisher"],
                "Vein Ripper": ["finisher"],
                "Nowhere to Run": ["cheap_removal", "eot_instant"], "Shoot the Sheriff": ["cheap_removal", "eot_instant"],
                "Bitter Triumph": ["cheap_removal", "eot_instant"],
            },
            "playOrder": ["early_play", "cheap_removal", "engine", "payoff", "finisher"],
            "evalCardBonus": {"Unholy Annex // Ritual Chamber": 0.55, "Bloodletter of Aclazotz": 0.45, "Unstoppable Slasher": 0.45},
            "sideboard": {
                "vs_aggro": {"in": {"Deadly Cover-Up": 2, "Stab": 3}, "out": {"Duress": 3, "Hero's Downfall": 2}},
                "vs_control": {"in": {"Duress": 1, "Pilfer": 3}, "out": {"Nowhere to Run": 4}},
                "vs_graveyard": {"in": {"Strategic Betrayal": 3}, "out": {"Hero's Downfall": 2, "Duress": 1}},
            },
        },
    ),
    (
        "standard", "tier1-boros-auras", "Boros Auras", "meta", "aggro", "aggro",
        "Fast enchantment voltron backed by Sheltered by Ghosts",
        [
            (4, "Optimistic Scavenger"), (4, "Gremlin Tamer"), (4, "Emberheart Challenger"),
            (4, "Slickshot Show-Off"), (4, "Ethereal Armor"), (4, "Sheltered by Ghosts"),
            (4, "Shardmage's Rescue"), (4, "Feather of Flight"), (4, "Burst Lightning"),
            (2, "Lightning Helix"), (4, "Sunbillow Verge"), (4, "Inspiring Vantage"),
            (4, "Sacred Foundry"), (6, "Plains"), (4, "Mountain"),
        ],
        [(3, "Authority of the Consuls"), (3, "Get Lost"), (3, "Rest in Peace"), (3, "Elspeth's Smite"), (3, "Torch the Tower")],
        {
            "keyCards": ["Optimistic Scavenger", "Ethereal Armor", "Sheltered by Ghosts", "Shardmage's Rescue"],
            "roles": {
                "Optimistic Scavenger": ["early_play", "engine"], "Emberheart Challenger": ["early_play", "haste"],
                "Slickshot Show-Off": ["early_play", "payoff"], "Gremlin Tamer": ["engine", "payoff"],
                "Ethereal Armor": ["payoff", "prowess_trigger"], "Sheltered by Ghosts": ["cheap_removal", "payoff"],
                "Shardmage's Rescue": ["combat_trick"], "Burst Lightning": ["burn", "cheap_removal"],
            },
            "playOrder": ["early_play", "engine", "payoff", "burn"],
            "sideboard": {
                "vs_aggro": {"in": {"Authority of the Consuls": 3, "Elspeth's Smite": 2}, "out": {"Feather of Flight": 3, "Gremlin Tamer": 2}},
                "vs_control": {"in": {"Get Lost": 2}, "out": {"Burst Lightning": 2}},
                "vs_graveyard": {"in": {"Rest in Peace": 3}, "out": {"Feather of Flight": 3}},
            },
        },
    ),
    (
        "standard", "tier1-orzhov-control", "Orzhov Control", "meta", "control", "control",
        "Sweepers, token engines, and planeswalker inevitability",
        [
            (4, "Caretaker's Talent"), (3, "Enduring Innocence"), (3, "Beza, the Bounding Spring"),
            (2, "Elspeth, Storm Slayer"), (3, "Deep-Cavern Bat"), (4, "Get Lost"),
            (4, "Shoot the Sheriff"), (4, "Nowhere to Run"), (3, "Day of Judgment"),
            (3, "Duress"), (2, "Authority of the Consuls"), (4, "Bleachbone Verge"),
            (4, "Godless Shrine"), (4, "Concealed Courtyard"), (2, "Fountainport"),
            (6, "Plains"), (5, "Swamp"),
        ],
        [(3, "Rest in Peace"), (1, "Day of Judgment"), (1, "Duress"), (3, "Pilfer"), (3, "Strategic Betrayal"), (2, "Withering Torment"), (2, "Elspeth's Smite")],
        {
            "keyCards": ["Caretaker's Talent", "Day of Judgment", "Beza, the Bounding Spring", "Elspeth, Storm Slayer"],
            "roles": {
                "Caretaker's Talent": ["engine"], "Enduring Innocence": ["engine"],
                "Beza, the Bounding Spring": ["payoff"], "Elspeth, Storm Slayer": ["finisher", "engine"],
                "Day of Judgment": ["sweeper"], "Get Lost": ["cheap_removal", "eot_instant"],
                "Shoot the Sheriff": ["cheap_removal", "eot_instant"], "Nowhere to Run": ["cheap_removal", "eot_instant"],
            },
            "cardRules": [
                {"card": "Day of Judgment", "castOnlyIf": {"or": [{"netOppCreaturesAtLeast": 1}, {"oppPowerAtLeast": 5}, {"myLifeAtMost": 9}]}},
            ],
            "evalCardBonus": {"Caretaker's Talent": 0.55, "Enduring Innocence": 0.40},
            "sideboard": {
                "vs_aggro": {"in": {"Day of Judgment": 1, "Elspeth's Smite": 2}, "out": {"Duress": 3}},
                "vs_control": {"in": {"Duress": 1, "Pilfer": 3}, "out": {"Nowhere to Run": 4}},
                "vs_graveyard": {"in": {"Rest in Peace": 3}, "out": {"Authority of the Consuls": 2, "Nowhere to Run": 1}},
            },
        },
    ),
    (
        "standard", "tier1-mono-red-burn", "Mono-Red Burn", "meta", "burn", "burn",
        "Low-curve haste creatures and relentless direct burn",
        [
            (4, "Clockwork Percussionist"), (4, "Hired Claw"), (4, "Emberheart Challenger"),
            (4, "Slickshot Show-Off"), (4, "Razorkin Needlehead"), (2, "Sunspine Lynx"),
            (4, "Boltwave"), (4, "Burst Lightning"), (4, "Lightning Strike"),
            (4, "Shock"), (2, "Torch the Tower"), (20, "Mountain"),
        ],
        [(2, "Sunspine Lynx"), (2, "Torch the Tower"), (4, "Abrade"), (3, "Scorching Dragonfire"), (4, "Soul-Guide Lantern")],
        {
            "keyCards": ["Boltwave", "Slickshot Show-Off", "Emberheart Challenger", "Sunspine Lynx"],
            "roles": {
                "Clockwork Percussionist": ["early_play", "haste"], "Hired Claw": ["early_play"],
                "Emberheart Challenger": ["early_play", "haste"], "Slickshot Show-Off": ["early_play", "payoff"],
                "Razorkin Needlehead": ["early_play", "engine"], "Sunspine Lynx": ["finisher"],
                "Boltwave": ["burn", "prowess_trigger"], "Burst Lightning": ["burn", "prowess_trigger"],
                "Lightning Strike": ["burn", "prowess_trigger"], "Shock": ["burn", "prowess_trigger"],
            },
            "playOrder": ["early_play", "payoff", "prowess_trigger", "burn", "finisher"],
            "sideboard": {
                "vs_aggro": {"in": {"Torch the Tower": 2, "Abrade": 3}, "out": {"Boltwave": 3, "Sunspine Lynx": 2}},
                "vs_control": {"in": {"Sunspine Lynx": 2, "Scorching Dragonfire": 3}, "out": {"Shock": 3, "Torch the Tower": 2}},
                "vs_graveyard": {"in": {"Soul-Guide Lantern": 4}, "out": {"Shock": 2, "Torch the Tower": 2}},
            },
        },
    ),
    (
        "standard", "tier1-selesnya-tokens", "Selesnya Tokens", "meta", "midrange", "midrange",
        "Mana elves into Caretaker's Talent and Overlord swarms",
        [
            (4, "Llanowar Elves"), (4, "Enduring Innocence"), (4, "Caretaker's Talent"),
            (4, "Overlord of the Mistmoors"), (3, "Beza, the Bounding Spring"), (3, "Elspeth, Storm Slayer"),
            (4, "Get Lost"), (3, "Authority of the Consuls"), (3, "Sheltered by Ghosts"),
            (2, "Enduring Vitality"), (4, "Hushwood Verge"), (4, "Temple Garden"),
            (4, "Lush Portico"), (2, "Fountainport"), (7, "Plains"), (5, "Forest"),
        ],
        [(3, "Rest in Peace"), (3, "Keen-Eyed Curator"), (3, "Day of Judgment"), (3, "Elspeth's Smite"), (3, "Pick Your Poison")],
        {
            "keyCards": ["Caretaker's Talent", "Overlord of the Mistmoors", "Elspeth, Storm Slayer", "Llanowar Elves"],
            "roles": {
                "Llanowar Elves": ["mana_dork", "early_play"], "Enduring Vitality": ["mana_dork", "engine"],
                "Caretaker's Talent": ["engine"], "Enduring Innocence": ["engine"],
                "Overlord of the Mistmoors": ["finisher"], "Elspeth, Storm Slayer": ["finisher", "engine"],
                "Get Lost": ["cheap_removal", "eot_instant"],
            },
            "playOrder": ["mana_dork", "engine", "payoff", "finisher"],
            "sideboard": {
                "vs_aggro": {"in": {"Elspeth's Smite": 3, "Day of Judgment": 2}, "out": {"Overlord of the Mistmoors": 2, "Enduring Vitality": 2, "Caretaker's Talent": 1}},
                "vs_graveyard": {"in": {"Rest in Peace": 3, "Keen-Eyed Curator": 2}, "out": {"Authority of the Consuls": 3, "Sheltered by Ghosts": 2}},
            },
        },
    ),
    (
        "standard", "tier1-simic-tempo", "Simic Terror Tempo", "meta", "tempo", "tempo",
        "Cantrip velocity into cheap Tolarian Terrors and counters",
        [
            (4, "Tolarian Terror"), (4, "Eddymurk Crab"), (4, "Abhorrent Oculus"),
            (3, "Brineborn Cutthroat"), (4, "Opt"), (4, "Deduce"),
            (4, "Sleight of Hand"), (4, "Into the Flood Maw"), (4, "Phantom Interference"),
            (3, "Spell Pierce"), (2, "Stock Up"), (4, "Willowrush Verge"),
            (4, "Breeding Pool"), (4, "Botanical Sanctum"), (6, "Island"), (2, "Forest"),
        ],
        [(3, "Negate"), (3, "Disdainful Stroke"), (3, "Pick Your Poison"), (3, "Soul-Guide Lantern"), (3, "Unsummon")],
        {
            "keyCards": ["Tolarian Terror", "Eddymurk Crab", "Abhorrent Oculus", "Phantom Interference"],
            "roles": {
                "Opt": ["draw", "eot_instant"], "Deduce": ["draw", "eot_instant"],
                "Into the Flood Maw": ["cheap_removal", "eot_instant"],
                "Phantom Interference": ["counter"], "Spell Pierce": ["counter"],
                "Brineborn Cutthroat": ["early_play", "eot_instant"],
                "Abhorrent Oculus": ["engine", "finisher"], "Tolarian Terror": ["finisher"], "Eddymurk Crab": ["finisher", "eot_instant"],
            },
            "playOrder": ["early_play", "engine", "draw", "finisher"],
            "sideboard": {
                "vs_aggro": {"in": {"Unsummon": 3}, "out": {"Spell Pierce": 3}},
                "vs_control": {"in": {"Negate": 3, "Disdainful Stroke": 2}, "out": {"Into the Flood Maw": 4, "Eddymurk Crab": 1}},
                "vs_graveyard": {"in": {"Soul-Guide Lantern": 3}, "out": {"Spell Pierce": 3}},
            },
        },
    ),
    (
        "standard", "tier1-golgari-demons", "Golgari Demons", "meta", "midrange", "midrange",
        "Green ramp and Sentinel into Unholy Annex demons",
        [
            (4, "Llanowar Elves"), (4, "Deep-Cavern Bat"), (4, "Preacher of the Schism"),
            (4, "Unstoppable Slasher"), (3, "Sentinel of the Nameless City"), (3, "Bloodletter of Aclazotz"),
            (4, "Unholy Annex // Ritual Chamber"), (4, "Shoot the Sheriff"), (3, "Nowhere to Run"),
            (2, "Maelstrom Pulse"), (2, "Bitter Triumph"), (4, "Wastewood Verge"),
            (4, "Blooming Marsh"), (4, "Overgrown Tomb"), (6, "Swamp"), (5, "Forest"),
        ],
        [(4, "Duress"), (3, "Keen-Eyed Curator"), (3, "Strategic Betrayal"), (3, "Pick Your Poison"), (2, "Stab")],
        {
            "keyCards": ["Unholy Annex // Ritual Chamber", "Unstoppable Slasher", "Sentinel of the Nameless City", "Llanowar Elves"],
            "roles": {
                "Llanowar Elves": ["mana_dork", "early_play"], "Deep-Cavern Bat": ["early_play", "engine"],
                "Preacher of the Schism": ["engine", "payoff"], "Unstoppable Slasher": ["payoff"],
                "Unholy Annex // Ritual Chamber": ["engine", "payoff"], "Bloodletter of Aclazotz": ["finisher"],
                "Shoot the Sheriff": ["cheap_removal", "eot_instant"], "Nowhere to Run": ["cheap_removal", "eot_instant"],
            },
            "playOrder": ["mana_dork", "early_play", "engine", "payoff", "finisher"],
            "sideboard": {
                "vs_aggro": {"in": {"Stab": 2}, "out": {"Bitter Triumph": 2}},
                "vs_control": {"in": {"Duress": 4}, "out": {"Nowhere to Run": 3, "Maelstrom Pulse": 1}},
                "vs_graveyard": {"in": {"Keen-Eyed Curator": 3, "Strategic Betrayal": 2}, "out": {"Nowhere to Run": 3, "Bitter Triumph": 2}},
            },
        },
    ),
    (
        "standard", "tier1-jeskai-control", "Jeskai Control", "meta", "control", "control",
        "Counterspells, Lightning Helix stabilization, and sweepers",
        [
            (4, "No More Lies"), (3, "Three Steps Ahead"), (4, "Lightning Helix"),
            (3, "Get Lost"), (3, "Torch the Tower"), (3, "Day of Judgment"),
            (4, "Stock Up"), (3, "Consult the Star Charts"), (3, "Beza, the Bounding Spring"),
            (2, "Elspeth, Storm Slayer"), (2, "The Theorist, Jace Beleren"),
            (4, "Hallowed Fountain"), (4, "Steam Vents"), (4, "Sacred Foundry"),
            (4, "Floodfarm Verge"), (4, "Riverpyre Verge"), (3, "Island"), (2, "Plains"), (1, "Mountain"),
        ],
        [(2, "Rest in Peace"), (3, "Disdainful Stroke"), (3, "Negate"), (2, "Abrade"), (1, "Day of Judgment"), (2, "Authority of the Consuls"), (2, "Elspeth's Smite")],
        {
            "keyCards": ["No More Lies", "Lightning Helix", "Day of Judgment", "The Theorist, Jace Beleren"],
            "roles": {
                "No More Lies": ["counter"], "Three Steps Ahead": ["counter", "draw", "eot_instant"],
                "Lightning Helix": ["cheap_removal", "burn", "eot_instant"], "Get Lost": ["cheap_removal", "eot_instant"],
                "Day of Judgment": ["sweeper"], "Consult the Star Charts": ["draw", "eot_instant"],
                "Stock Up": ["draw"], "The Theorist, Jace Beleren": ["engine", "finisher"], "Elspeth, Storm Slayer": ["finisher"],
            },
            "cardRules": [
                {"card": "Three Steps Ahead", "holdForStep": "opp_end_step", "modePriority": [{"if": {"oppSpellOnStack": True}, "modes": [0]}, {"default": True, "modes": [2]}]},
                {"card": "Consult the Star Charts", "holdForStep": "opp_end_step", "unless": {"and": [{"myLandsAtMost": 3}, {"noLandInHand": True}]}},
                {"card": "Day of Judgment", "castOnlyIf": {"or": [{"netOppCreaturesAtLeast": 1}, {"oppPowerAtLeast": 5}, {"myLifeAtMost": 10}]}},
            ],
            "sideboard": {
                "vs_aggro": {"in": {"Day of Judgment": 1, "Authority of the Consuls": 2, "Elspeth's Smite": 2}, "out": {"Three Steps Ahead": 2, "Stock Up": 2, "Consult the Star Charts": 1}},
                "vs_control": {"in": {"Disdainful Stroke": 3, "Negate": 3}, "out": {"Day of Judgment": 3, "Torch the Tower": 3}},
                "vs_graveyard": {"in": {"Rest in Peace": 2}, "out": {"Torch the Tower": 2}},
            },
        },
    ),
    (
        "standard", "tier1-esper-midrange", "Esper Tempo", "meta", "tempo", "tempo",
        "Flash flyers, Kaito disruption, and versatile white-black removal",
        [
            (4, "Floodpits Drowner"), (4, "Spyglass Siren"), (4, "Deep-Cavern Bat"),
            (3, "Preacher of the Schism"), (3, "Enduring Curiosity"), (3, "Kaito, Bane of Nightmares"),
            (4, "No More Lies"), (4, "Shoot the Sheriff"), (3, "Get Lost"),
            (3, "Nowhere to Run"), (4, "Watery Grave"), (4, "Hallowed Fountain"),
            (4, "Godless Shrine"), (4, "Gloomlake Verge"), (4, "Floodfarm Verge"),
            (2, "Island"), (2, "Swamp"), (1, "Plains"),
        ],
        [(3, "Duress"), (3, "Negate"), (3, "Rest in Peace"), (3, "Disdainful Stroke"), (3, "Elspeth's Smite")],
        {
            "keyCards": ["Kaito, Bane of Nightmares", "Enduring Curiosity", "Floodpits Drowner", "No More Lies"],
            "roles": {
                "Spyglass Siren": ["early_play"], "Floodpits Drowner": ["early_play", "eot_instant", "cheap_removal"],
                "Deep-Cavern Bat": ["early_play", "engine"], "Preacher of the Schism": ["engine", "payoff"],
                "Enduring Curiosity": ["engine", "eot_instant"], "Kaito, Bane of Nightmares": ["engine", "finisher"],
                "No More Lies": ["counter"], "Shoot the Sheriff": ["cheap_removal", "eot_instant"], "Get Lost": ["cheap_removal", "eot_instant"],
            },
            "playOrder": ["early_play", "engine", "cheap_removal", "finisher"],
            "sideboard": {
                "vs_aggro": {"in": {"Elspeth's Smite": 3}, "out": {"No More Lies": 3}},
                "vs_control": {"in": {"Duress": 3, "Negate": 2}, "out": {"Nowhere to Run": 3, "Shoot the Sheriff": 2}},
                "vs_graveyard": {"in": {"Rest in Peace": 3}, "out": {"Nowhere to Run": 3}},
            },
        },
    ),
    (
        "standard", "tier1-gruul-prowess", "Gruul Prowess", "meta", "aggro", "aggro",
        "Explosive red prowess creatures accelerated by Llanowar Elves",
        [
            (4, "Llanowar Elves"), (4, "Clockwork Percussionist"), (4, "Hired Claw"),
            (4, "Emberheart Challenger"), (4, "Slickshot Show-Off"), (3, "Bristly Bill, Spine Sower"),
            (4, "Burst Lightning"), (4, "Lightning Strike"), (4, "Shock"),
            (3, "Hunter's Talent"), (4, "Thornspire Verge"), (4, "Stomping Ground"),
            (4, "Commercial District"), (6, "Mountain"), (4, "Forest"),
        ],
        [(3, "Abrade"), (3, "Pick Your Poison"), (3, "Keen-Eyed Curator"), (3, "Scorching Dragonfire"), (3, "Torch the Tower")],
        {
            "keyCards": ["Emberheart Challenger", "Slickshot Show-Off", "Bristly Bill, Spine Sower", "Burst Lightning"],
            "roles": {
                "Llanowar Elves": ["mana_dork", "early_play"], "Clockwork Percussionist": ["early_play", "haste"],
                "Hired Claw": ["early_play"], "Emberheart Challenger": ["early_play", "haste"],
                "Slickshot Show-Off": ["early_play", "payoff"], "Bristly Bill, Spine Sower": ["engine", "payoff"],
                "Burst Lightning": ["burn", "prowess_trigger"], "Lightning Strike": ["burn", "prowess_trigger"],
            },
            "playOrder": ["early_play", "mana_dork", "payoff", "burn"],
            "sideboard": {
                "vs_aggro": {"in": {"Torch the Tower": 3, "Abrade": 2}, "out": {"Hunter's Talent": 3, "Bristly Bill, Spine Sower": 2}},
                "vs_control": {"in": {"Scorching Dragonfire": 3}, "out": {"Shock": 3}},
                "vs_graveyard": {"in": {"Keen-Eyed Curator": 3}, "out": {"Shock": 3}},
            },
        },
    ),
    (
        "standard", "tier2-mono-white-tokens", "Mono-White Control", "tier2", "control", "control",
        "Caretaker's Talent value engine behind white sweepers",
        [
            (4, "Caretaker's Talent"), (4, "Enduring Innocence"), (4, "Beza, the Bounding Spring"),
            (3, "Elspeth, Storm Slayer"), (3, "Overlord of the Mistmoors"), (4, "Get Lost"),
            (4, "Day of Judgment"), (4, "Authority of the Consuls"), (4, "Elspeth's Smite"),
            (3, "Fountainport"), (2, "Demolition Field"), (21, "Plains"),
        ],
        [(4, "Rest in Peace"), (3, "Sheltered by Ghosts"), (4, "Soul-Guide Lantern"), (4, "Disenchant")],
        {"roles": {"Caretaker's Talent": ["engine"], "Day of Judgment": ["sweeper"], "Elspeth, Storm Slayer": ["finisher"]}},
    ),
    (
        "standard", "tier2-dimir-reanimator", "Dimir Reanimator", "tier2", "graveyard", "graveyard",
        "Self-mill and discard fueling Zombify into Valgavoth and Sire of Seven Deaths",
        [
            (4, "Zombify"), (4, "Valgavoth, Terror Eater"), (3, "Sire of Seven Deaths"),
            (4, "Kiora, the Rising Tide"), (4, "Overlord of the Balemurk"), (4, "Chart a Course"),
            (4, "Opt"), (4, "Shoot the Sheriff"), (4, "Nowhere to Run"),
            (2, "Duress"), (4, "Watery Grave"), (4, "Gloomlake Verge"),
            (4, "Undercity Sewers"), (6, "Swamp"), (5, "Island"),
        ],
        [(2, "Duress"), (3, "Negate"), (3, "Disdainful Stroke"), (4, "Bitter Triumph"), (3, "Pilfer")],
        {
            "keyCards": ["Zombify", "Valgavoth, Terror Eater", "Sire of Seven Deaths", "Overlord of the Balemurk"],
            "roles": {
                "Valgavoth, Terror Eater": ["reanimate_target", "finisher"], "Sire of Seven Deaths": ["reanimate_target", "finisher"],
                "Zombify": ["reanimator", "combo_piece"], "Kiora, the Rising Tide": ["enabler"],
                "Chart a Course": ["enabler", "draw"], "Overlord of the Balemurk": ["enabler", "payoff"],
            },
            "discardRolesFirst": ["reanimate_target"],
            "neverDiscardRoles": ["reanimator"],
            "graveyardRoleBonus": {"reanimate_target": 0.65},
            "cardRules": [{"card": "Zombify", "castOnlyIf": {"graveyardHasRole": "reanimate_target"}}],
        },
    ),
    (
        "standard", "tier2-rakdos-midrange", "Rakdos Demons", "tier2", "midrange", "midrange",
        "Red burn pressure paired with Unholy Annex demons",
        [
            (4, "Deep-Cavern Bat"), (4, "Preacher of the Schism"), (4, "Razorkin Needlehead"),
            (4, "Unstoppable Slasher"), (3, "Unholy Annex // Ritual Chamber"), (3, "Bloodletter of Aclazotz"),
            (4, "Burst Lightning"), (4, "Shoot the Sheriff"), (3, "Nowhere to Run"),
            (3, "Torch the Tower"), (4, "Blood Crypt"), (4, "Blazemire Verge"),
            (4, "Raucous Theater"), (7, "Swamp"), (5, "Mountain"),
        ],
        [(4, "Duress"), (3, "Abrade"), (3, "Strategic Betrayal"), (3, "Sunspine Lynx"), (2, "Hero's Downfall")],
        {"roles": {"Unholy Annex // Ritual Chamber": ["engine"], "Unstoppable Slasher": ["payoff"], "Bloodletter of Aclazotz": ["finisher"]}},
    ),
    (
        "standard", "tier2-azorius-enchantments", "Azorius Enchantments", "tier2", "tempo", "tempo",
        "Enchantment creatures drawing cards with Entity Tracker",
        [
            (4, "Optimistic Scavenger"), (4, "Inquisitive Glimmer"), (4, "Gremlin Tamer"),
            (4, "Entity Tracker"), (4, "Ethereal Armor"), (4, "Sheltered by Ghosts"),
            (4, "Shardmage's Rescue"), (4, "Feather of Flight"), (4, "Into the Flood Maw"),
            (2, "No More Lies"), (4, "Hallowed Fountain"), (4, "Floodfarm Verge"),
            (4, "Meticulous Archive"), (6, "Plains"), (4, "Island"),
        ],
        [(2, "No More Lies"), (3, "Negate"), (3, "Rest in Peace"), (4, "Authority of the Consuls"), (3, "Get Lost")],
        {"roles": {"Optimistic Scavenger": ["early_play", "engine"], "Entity Tracker": ["engine", "eot_instant"], "No More Lies": ["counter"]}},
    ),
    (
        "standard", "tier2-mono-green-elves", "Mono-Green Elves", "tier2", "tribal", "tribal",
        "Elvish mana acceleration snowballing with lords",
        [
            (4, "Llanowar Elves"), (4, "Elvish Archdruid"), (4, "Dwynen, Gilt-Leaf Daen"),
            (4, "Imperious Perfect"), (4, "Elvish Regrower"), (3, "Tyvar, the Pummeler"),
            (3, "Outcaster Trailblazer"), (4, "Bristly Bill, Spine Sower"), (4, "Hunter's Talent"),
            (4, "Enduring Vitality"), (2, "Sire of Seven Deaths"), (20, "Forest"),
        ],
        [(4, "Pick Your Poison"), (4, "Keen-Eyed Curator"), (4, "Pawpatch Formation"), (3, "Soul-Guide Lantern")],
        {"roles": {"Llanowar Elves": ["mana_dork", "early_play"], "Elvish Archdruid": ["mana_dork", "lord"], "Dwynen, Gilt-Leaf Daen": ["lord"], "Imperious Perfect": ["lord"]}},
    ),
    (
        "standard", "tier2-boros-convoke", "Boros Convoke", "tier2", "aggro", "aggro",
        "Go-wide token swarms into Imodane's Recruiter alpha strikes",
        [
            (4, "Novice Inspector"), (4, "Clockwork Percussionist"), (4, "Warden of the Inner Sky"),
            (4, "Resolute Reinforcements"), (4, "Imodane's Recruiter"), (4, "Warleader's Call"),
            (4, "Case of the Gateway Express"), (3, "Anim Pakal, Thousandth Moon"), (4, "Burst Lightning"),
            (3, "Lightning Helix"), (4, "Sacred Foundry"), (4, "Sunbillow Verge"),
            (4, "Inspiring Vantage"), (6, "Plains"), (4, "Mountain"),
        ],
        [(3, "Authority of the Consuls"), (3, "Get Lost"), (3, "Rest in Peace"), (3, "Abrade"), (3, "Elspeth's Smite")],
        {"roles": {"Novice Inspector": ["early_play", "fodder"], "Warleader's Call": ["lord", "engine"], "Imodane's Recruiter": ["finisher", "haste"]}},
    ),
    (
        "standard", "rogue-mono-blue-tempo", "Mono-Blue Tempo", "rogue", "tempo", "tempo",
        "Flash threats and cheap countermagic",
        [
            (4, "Spectral Sailor"), (4, "Brineborn Cutthroat"), (4, "Floodpits Drowner"),
            (3, "Malcolm, Alluring Scoundrel"), (4, "Eddymurk Crab"), (3, "Tolarian Terror"),
            (4, "Opt"), (4, "Deduce"), (4, "Into the Flood Maw"),
            (4, "Phantom Interference"), (2, "Negate"), (20, "Island"),
        ],
        [(2, "Negate"), (4, "Disdainful Stroke"), (4, "Essence Scatter"), (2, "Spell Pierce"), (3, "Unsummon")],
        {"roles": {"Spectral Sailor": ["early_play", "eot_instant"], "Floodpits Drowner": ["early_play", "eot_instant"], "Phantom Interference": ["counter"]}},
    ),
    (
        "standard", "rogue-orzhov-lifegain", "Orzhov Lifegain", "rogue", "tribal", "tribal",
        "Lifegain triggers growing Ajani's Pridemate and Angels",
        [
            (4, "Authority of the Consuls"), (4, "Ajani's Pridemate"), (4, "Youthful Valkyrie"),
            (4, "Giada, Font of Hope"), (4, "Deep-Cavern Bat"), (4, "Enduring Innocence"),
            (3, "Lyra Dawnbringer"), (4, "Sheltered by Ghosts"), (4, "Shoot the Sheriff"),
            (3, "Get Lost"), (4, "Godless Shrine"), (4, "Bleachbone Verge"),
            (4, "Concealed Courtyard"), (6, "Plains"), (4, "Swamp"),
        ],
        [(4, "Duress"), (4, "Rest in Peace"), (4, "Nowhere to Run"), (3, "Day of Judgment")],
        {"roles": {"Giada, Font of Hope": ["mana_dork", "lord"], "Enduring Innocence": ["engine"]}},
    ),
    (
        "standard", "rogue-mono-red-goblins", "Mono-Red Goblins", "rogue", "tribal", "tribal",
        "Goblin token swarm led by Krenko and Howlsquad Heavy",
        [
            (4, "Searslicer Goblin"), (4, "Fanatical Firebrand"), (4, "Dropkick Bomber"),
            (4, "Krenko, Mob Boss"), (4, "Howlsquad Heavy"), (4, "Hired Claw"),
            (4, "Goblin Surprise"), (4, "Burst Lightning"), (4, "Lightning Strike"),
            (4, "Shock"), (20, "Mountain"),
        ],
        [(4, "Abrade"), (4, "Torch the Tower"), (4, "Scorching Dragonfire"), (3, "Soul-Guide Lantern")],
        {"roles": {"Dropkick Bomber": ["lord", "engine"], "Krenko, Mob Boss": ["engine", "finisher"]}},
    ),
    (
        "standard", "rogue-simic-ramp", "Simic Overlord Ramp", "rogue", "ramp", "ramp",
        "Mana elves and Overlord of the Hauntwoods into massive finishers",
        [
            (4, "Llanowar Elves"), (4, "Enduring Vitality"), (4, "Overlord of the Hauntwoods"),
            (4, "Overlord of the Floodpits"), (3, "Sire of Seven Deaths"), (3, "Bristly Bill, Spine Sower"),
            (4, "Stock Up"), (4, "Into the Flood Maw"), (3, "Phantom Interference"),
            (3, "Kiora, the Rising Tide"), (4, "Breeding Pool"), (4, "Willowrush Verge"),
            (4, "Botanical Sanctum"), (7, "Forest"), (5, "Island"),
        ],
        [(4, "Negate"), (4, "Disdainful Stroke"), (4, "Pick Your Poison"), (3, "Keen-Eyed Curator")],
        {"roles": {"Llanowar Elves": ["mana_dork", "early_play"], "Overlord of the Hauntwoods": ["ramp", "finisher"], "Sire of Seven Deaths": ["finisher"]}},
    ),

    # ===================== PIONEER (+20 -> 42 total) =====================
    (
        "pioneer", "tier1-rakdos-midrange", "Rakdos Midrange", "meta", "midrange", "midrange",
        "Thoughtseize, Fable of the Mirror-Breaker, and Sheoldred",
        [
            (4, "Thoughtseize"), (4, "Fatal Push"), (4, "Bloodtithe Harvester"),
            (4, "Bonecrusher Giant"), (4, "Fable of the Mirror-Breaker"), (3, "Graveyard Trespasser"),
            (4, "Sheoldred, the Apocalypse"), (2, "Kroxa, Titan of Death's Hunger"), (3, "Dreadbore"),
            (2, "Kolaghan's Command"), (2, "Go for the Throat"), (4, "Blood Crypt"),
            (4, "Blackcleave Cliffs"), (4, "Blazemire Verge"), (2, "Mutavault"),
            (2, "Castle Locthwain"), (5, "Swamp"), (3, "Mountain"),
        ],
        [(3, "Duress"), (3, "Unlicensed Hearse"), (3, "Abrade"), (3, "Anger of the Gods"), (3, "Go Blank")],
        {
            "keyCards": ["Fable of the Mirror-Breaker", "Sheoldred, the Apocalypse", "Thoughtseize", "Bloodtithe Harvester"],
            "roles": {
                "Thoughtseize": ["early_play"], "Bloodtithe Harvester": ["early_play", "cheap_removal"],
                "Fable of the Mirror-Breaker": ["engine", "payoff"], "Bonecrusher Giant": ["cheap_removal", "payoff"],
                "Sheoldred, the Apocalypse": ["finisher", "engine"], "Fatal Push": ["cheap_removal", "eot_instant"],
            },
            "playOrder": ["early_play", "cheap_removal", "engine", "payoff", "finisher"],
            "evalCardBonus": {"Fable of the Mirror-Breaker": 0.60, "Sheoldred, the Apocalypse": 0.65},
            "sideboard": {
                "vs_aggro": {"in": {"Anger of the Gods": 3, "Abrade": 2}, "out": {"Thoughtseize": 3, "Kolaghan's Command": 2}},
                "vs_control": {"in": {"Duress": 3, "Go Blank": 3}, "out": {"Fatal Push": 4, "Go for the Throat": 2}},
                "vs_graveyard": {"in": {"Unlicensed Hearse": 3, "Go Blank": 2}, "out": {"Go for the Throat": 2, "Dreadbore": 2, "Kolaghan's Command": 1}},
            },
        },
    ),
    (
        "pioneer", "tier1-rakdos-sacrifice", "Rakdos Sacrifice", "meta", "sacrifice", "sacrifice",
        "Cat-Oven loop and Mayhem Devil triggers",
        [
            (4, "Cauldron Familiar"), (4, "Witch's Oven"), (4, "Mayhem Devil"),
            (4, "Voldaren Epicure"), (4, "Bloodtithe Harvester"), (4, "Deadly Dispute"),
            (4, "Claim the Firstborn"), (4, "Fatal Push"), (3, "Thoughtseize"),
            (3, "Oni-Cult Anvil"), (4, "Blood Crypt"), (4, "Blackcleave Cliffs"),
            (4, "Blazemire Verge"), (6, "Swamp"), (4, "Mountain"),
        ],
        [(3, "Duress"), (3, "Go Blank"), (3, "Unlicensed Hearse"), (3, "Abrade"), (3, "Anger of the Gods")],
        {
            "keyCards": ["Mayhem Devil", "Cauldron Familiar", "Witch's Oven", "Deadly Dispute"],
            "roles": {
                "Cauldron Familiar": ["fodder", "early_play"], "Voldaren Epicure": ["fodder", "early_play"],
                "Witch's Oven": ["sac_outlet", "early_play"], "Oni-Cult Anvil": ["sac_outlet", "engine"],
                "Mayhem Devil": ["payoff", "engine"], "Deadly Dispute": ["draw", "eot_instant"],
                "Fatal Push": ["cheap_removal", "eot_instant"],
            },
            "playOrder": ["early_play", "fodder", "sac_outlet", "engine", "payoff"],
            "evalCardBonus": {"Mayhem Devil": 0.65, "Witch's Oven": 0.40},
            "sideboard": {
                "vs_aggro": {"in": {"Anger of the Gods": 2, "Abrade": 2}, "out": {"Thoughtseize": 3, "Oni-Cult Anvil": 1}},
                "vs_control": {"in": {"Duress": 3, "Go Blank": 3}, "out": {"Claim the Firstborn": 4, "Fatal Push": 2}},
                "vs_graveyard": {"in": {"Unlicensed Hearse": 3, "Go Blank": 2}, "out": {"Claim the Firstborn": 4, "Oni-Cult Anvil": 1}},
            },
        },
    ),
    (
        "pioneer", "tier1-selesnya-angels", "Selesnya Angels", "meta", "tribal", "tribal",
        "Lifegain Angel snowball with Collected Company and Kayla's Reconstruction",
        [
            (4, "Bishop of Wings"), (4, "Giada, Font of Hope"), (4, "Youthful Valkyrie"),
            (4, "Righteous Valkyrie"), (4, "Resplendent Angel"), (4, "Skyclave Apparition"),
            (4, "Collected Company"), (3, "Kayla's Reconstruction"), (3, "Llanowar Elves"),
            (2, "Inspiring Overseer"), (4, "Temple Garden"), (4, "Razorverge Thicket"),
            (4, "Brushland"), (4, "Hushwood Verge"), (6, "Plains"), (2, "Forest"),
        ],
        [(3, "Rest in Peace"), (3, "Portable Hole"), (3, "Knight of Autumn"), (3, "Get Lost"), (3, "Damping Sphere")],
        {
            "keyCards": ["Righteous Valkyrie", "Collected Company", "Bishop of Wings", "Resplendent Angel"],
            "roles": {
                "Llanowar Elves": ["mana_dork", "early_play"], "Giada, Font of Hope": ["mana_dork", "lord"],
                "Bishop of Wings": ["engine", "early_play"], "Righteous Valkyrie": ["lord", "payoff"],
                "Resplendent Angel": ["payoff", "finisher"], "Collected Company": ["eot_instant", "payoff"],
            },
            "cardRules": [{"card": "Collected Company", "holdForStep": "opp_end_step", "unless": {"and": [{"notMyTurn": True}, {"oppCreaturesAtLeast": 1}]}}],
            "evalCardBonus": {"Righteous Valkyrie": 0.55, "Bishop of Wings": 0.45},
            "sideboard": {
                "vs_aggro": {"in": {"Portable Hole": 3}, "out": {"Kayla's Reconstruction": 3}},
                "vs_graveyard": {"in": {"Rest in Peace": 3}, "out": {"Inspiring Overseer": 2, "Skyclave Apparition": 1}},
            },
        },
    ),
    (
        "pioneer", "tier1-mono-green-devotion", "Mono-Green Devotion", "meta", "ramp", "ramp",
        "Nykthos mana engine into Cavalier of Thorns and Storm the Festival",
        [
            (4, "Llanowar Elves"), (4, "Elvish Mystic"), (4, "Wolfwillow Haven"),
            (4, "Kiora, Behemoth Beckoner"), (4, "Old-Growth Troll"), (4, "Cavalier of Thorns"),
            (4, "Outcaster Trailblazer"), (4, "Storm the Festival"), (3, "Polukranos Reborn"),
            (2, "Ghalta, Stampede Tyrant"), (4, "Nykthos, Shrine to Nyx"), (19, "Forest"),
        ],
        [(4, "Pick Your Poison"), (4, "Keen-Eyed Curator"), (4, "Scavenging Ooze"), (3, "Damping Sphere")],
        {
            "keyCards": ["Nykthos, Shrine to Nyx", "Kiora, Behemoth Beckoner", "Cavalier of Thorns", "Storm the Festival"],
            "roles": {
                "Llanowar Elves": ["mana_dork", "early_play"], "Elvish Mystic": ["mana_dork", "early_play"],
                "Wolfwillow Haven": ["ramp", "early_play"], "Kiora, Behemoth Beckoner": ["engine", "ramp"],
                "Old-Growth Troll": ["payoff"], "Cavalier of Thorns": ["payoff", "ramp"], "Storm the Festival": ["finisher"],
            },
            "playOrder": ["mana_dork", "ramp", "engine", "payoff", "finisher"],
            "evalCardBonus": {"Kiora, Behemoth Beckoner": 0.50, "Nykthos, Shrine to Nyx": 0.50},
            "sideboard": {
                "vs_graveyard": {"in": {"Keen-Eyed Curator": 4}, "out": {"Polukranos Reborn": 2, "Ghalta, Stampede Tyrant": 2}},
            },
        },
    ),
    (
        "pioneer", "tier1-boros-heroic", "Boros Heroic", "meta", "aggro", "aggro",
        "Heroic creatures scaling fast with pump and protection",
        [
            (4, "Favored Hoplite"), (4, "Illuminator Virtuoso"), (4, "Clever Lumimancer"),
            (4, "Monastery Swiftspear"), (4, "Homestead Courage"), (4, "Defiant Strike"),
            (4, "Gods Willing"), (4, "Loran's Escape"), (4, "Monstrous Rage"),
            (4, "Reckless Rage"), (4, "Sacred Foundry"), (4, "Inspiring Vantage"),
            (4, "Sunbillow Verge"), (5, "Plains"), (3, "Mountain"),
        ],
        [(3, "Portable Hole"), (3, "Rest in Peace"), (3, "Abrade"), (3, "Rending Volley"), (3, "Authority of the Consuls")],
        {
            "keyCards": ["Illuminator Virtuoso", "Favored Hoplite", "Monstrous Rage", "Gods Willing"],
            "roles": {
                "Favored Hoplite": ["early_play", "payoff"], "Illuminator Virtuoso": ["payoff"],
                "Clever Lumimancer": ["early_play"], "Monastery Swiftspear": ["early_play", "haste"],
                "Gods Willing": ["combat_trick"], "Loran's Escape": ["combat_trick"],
                "Monstrous Rage": ["combat_trick", "prowess_trigger"], "Reckless Rage": ["cheap_removal"],
            },
            "playOrder": ["early_play", "payoff", "prowess_trigger"],
            "sideboard": {
                "vs_aggro": {"in": {"Portable Hole": 3, "Authority of the Consuls": 2}, "out": {"Defiant Strike": 3, "Loran's Escape": 2}},
                "vs_graveyard": {"in": {"Rest in Peace": 3}, "out": {"Reckless Rage": 3}},
            },
        },
    ),
    (
        "pioneer", "tier1-boros-convoke", "Boros Convoke", "meta", "aggro", "aggro",
        "Explosive artifact tokens into Imodane's Recruiter finishes",
        [
            (4, "Thraben Inspector"), (4, "Novice Inspector"), (4, "Voldaren Epicure"),
            (4, "Gleeful Demolition"), (4, "Warden of the Inner Sky"), (4, "Imodane's Recruiter"),
            (4, "Warleader's Call"), (4, "Case of the Gateway Express"), (4, "Play with Fire"),
            (2, "Anim Pakal, Thousandth Moon"), (4, "Sacred Foundry"), (4, "Inspiring Vantage"),
            (4, "Sunbillow Verge"), (6, "Plains"), (4, "Mountain"),
        ],
        [(3, "Portable Hole"), (3, "Rest in Peace"), (3, "Abrade"), (3, "Authority of the Consuls"), (3, "Get Lost")],
        {"roles": {"Thraben Inspector": ["early_play", "fodder"], "Warleader's Call": ["lord", "engine"], "Imodane's Recruiter": ["finisher"]}},
    ),
    (
        "pioneer", "tier1-azorius-spirits", "Azorius Spirits", "meta", "tempo", "tempo",
        "Flash flying lords, Spell Queller, and Curious Obsession",
        [
            (4, "Mausoleum Wanderer"), (4, "Spectral Sailor"), (4, "Rattlechains"),
            (4, "Supreme Phantom"), (3, "Shacklegeist"), (4, "Spell Queller"),
            (3, "Empyrean Eagle"), (2, "Selfless Spirit"), (4, "Curious Obsession"),
            (4, "Lofty Denial"), (2, "Slip Out the Back"), (4, "Hallowed Fountain"),
            (4, "Seachrome Coast"), (4, "Floodfarm Verge"), (6, "Island"), (4, "Plains"),
        ],
        [(3, "Mystical Dispute"), (3, "Rest in Peace"), (3, "Portable Hole"), (3, "Dovin's Veto"), (3, "Get Lost")],
        {
            "keyCards": ["Spell Queller", "Supreme Phantom", "Curious Obsession", "Lofty Denial"],
            "roles": {
                "Mausoleum Wanderer": ["early_play"], "Rattlechains": ["eot_instant", "engine"],
                "Supreme Phantom": ["lord"], "Empyrean Eagle": ["lord"],
                "Spell Queller": ["counter", "eot_instant"], "Lofty Denial": ["counter"],
                "Slip Out the Back": ["combat_trick", "eot_instant"],
            },
            "sideboard": {
                "vs_aggro": {"in": {"Portable Hole": 3}, "out": {"Lofty Denial": 3}},
                "vs_control": {"in": {"Mystical Dispute": 3, "Dovin's Veto": 3}, "out": {"Shacklegeist": 4, "Selfless Spirit": 2}},
                "vs_graveyard": {"in": {"Rest in Peace": 3}, "out": {"Slip Out the Back": 3}},
            },
        },
    ),
    (
        "pioneer", "tier1-gruul-vehicles", "Gruul Vehicles", "meta", "midrange", "midrange",
        "Mana elves into Bonecrusher Giant, Esika's Chariot, and Embercleave",
        [
            (4, "Llanowar Elves"), (4, "Elvish Mystic"), (4, "Bonecrusher Giant"),
            (4, "Lovestruck Beast"), (4, "Reckless Stormseeker"), (4, "Esika's Chariot"),
            (3, "Migloz, Maze Crusher"), (3, "Questing Beast"), (2, "Embercleave"),
            (4, "Play with Fire"), (2, "Abrade"), (4, "Stomping Ground"),
            (4, "Copperline Gorge"), (4, "Thornspire Verge"), (6, "Forest"), (4, "Mountain"),
        ],
        [(2, "Abrade"), (3, "Rending Volley"), (3, "Pick Your Poison"), (3, "Scavenging Ooze"), (4, "Unlicensed Hearse")],
        {
            "keyCards": ["Esika's Chariot", "Reckless Stormseeker", "Bonecrusher Giant", "Embercleave"],
            "roles": {"Llanowar Elves": ["mana_dork", "early_play"], "Elvish Mystic": ["mana_dork", "early_play"], "Esika's Chariot": ["payoff", "engine"], "Embercleave": ["finisher"]},
            "playOrder": ["mana_dork", "early_play", "payoff", "finisher"],
        },
    ),
    (
        "pioneer", "tier1-dimir-control", "Dimir Control", "meta", "control", "control",
        "Black hand disruption paired with counters, Memory Deluge, and Shark Typhoon",
        [
            (4, "Thoughtseize"), (4, "Fatal Push"), (4, "Drown in the Loch"),
            (4, "Consider"), (3, "Cling to Dust"), (3, "Sinister Sabotage"),
            (3, "Memory Deluge"), (3, "Shark Typhoon"), (2, "Narset, Parter of Veils"),
            (3, "Sheoldred, the Apocalypse"), (2, "Go for the Throat"), (4, "Watery Grave"),
            (4, "Darkslick Shores"), (4, "Gloomlake Verge"), (2, "Castle Locthwain"),
            (6, "Island"), (5, "Swamp"),
        ],
        [(3, "Duress"), (3, "Mystical Dispute"), (3, "Aether Gust"), (3, "Go Blank"), (3, "Unlicensed Hearse")],
        {
            "keyCards": ["Memory Deluge", "Shark Typhoon", "Drown in the Loch", "Sheoldred, the Apocalypse"],
            "roles": {
                "Drown in the Loch": ["counter", "cheap_removal", "eot_instant"],
                "Sinister Sabotage": ["counter"], "Memory Deluge": ["draw", "eot_instant"],
                "Fatal Push": ["cheap_removal", "eot_instant"], "Shark Typhoon": ["finisher", "eot_instant"],
                "Sheoldred, the Apocalypse": ["finisher", "engine"],
            },
            "cardRules": [{"card": "Memory Deluge", "holdForStep": "opp_end_step"}],
            "sideboard": {
                "vs_control": {"in": {"Duress": 3, "Mystical Dispute": 3}, "out": {"Fatal Push": 4, "Go for the Throat": 2}},
                "vs_graveyard": {"in": {"Unlicensed Hearse": 3, "Go Blank": 2}, "out": {"Narset, Parter of Veils": 2, "Go for the Throat": 2, "Consider": 1}},
            },
        },
    ),
    (
        "pioneer", "tier1-niv-to-light", "Five-Color Niv-to-Light", "meta", "control", "control",
        "Bring to Light toolbox and Niv-Mizzet Reborn value engine",
        [
            (4, "Niv-Mizzet Reborn"), (4, "Bring to Light"), (3, "Omnath, Locus of Creation"),
            (4, "Vanishing Verse"), (3, "Abrupt Decay"), (3, "Lightning Helix"),
            (3, "Dreadbore"), (2, "Kolaghan's Command"), (2, "Supreme Verdict"),
            (4, "Growth Spiral"), (2, "Fable of the Mirror-Breaker"),
            (4, "Fabled Passage"), (3, "Hallowed Fountain"), (3, "Watery Grave"),
            (3, "Blood Crypt"), (3, "Stomping Ground"), (3, "Temple Garden"),
            (2, "Breeding Pool"), (1, "Plains"), (1, "Island"), (1, "Swamp"), (1, "Mountain"), (1, "Forest"),
        ],
        [(3, "Thoughtseize"), (3, "Dovin's Veto"), (3, "Rest in Peace"), (3, "Knight of Autumn"), (3, "Unlicensed Hearse")],
        {
            "keyCards": ["Niv-Mizzet Reborn", "Bring to Light", "Omnath, Locus of Creation", "Vanishing Verse"],
            "roles": {"Growth Spiral": ["ramp", "eot_instant"], "Supreme Verdict": ["sweeper"], "Niv-Mizzet Reborn": ["finisher", "engine"], "Omnath, Locus of Creation": ["engine", "payoff"]},
        },
    ),
    (
        "pioneer", "tier2-dimir-rogues", "Dimir Rogues", "tier2", "tempo", "tempo",
        "Flash Rogues enabling Drown in the Loch and Into the Story",
        [
            (4, "Thieves' Guild Enforcer"), (4, "Soaring Thought-Thief"), (4, "Faerie Mastermind"),
            (3, "Nighthawk Scavenger"), (3, "Brazen Borrower"), (4, "Drown in the Loch"),
            (4, "Fatal Push"), (4, "Thoughtseize"), (4, "Into the Story"),
            (2, "Cling to Dust"), (4, "Watery Grave"), (4, "Darkslick Shores"),
            (4, "Gloomlake Verge"), (6, "Island"), (6, "Swamp"),
        ],
        [(3, "Mystical Dispute"), (3, "Duress"), (3, "Aether Gust"), (3, "Go Blank"), (3, "Unlicensed Hearse")],
        {"roles": {"Thieves' Guild Enforcer": ["early_play", "eot_instant"], "Soaring Thought-Thief": ["lord", "eot_instant"], "Drown in the Loch": ["counter", "cheap_removal"], "Into the Story": ["draw", "eot_instant"]}},
    ),
    (
        "pioneer", "tier2-mono-black-devotion", "Mono-Black Devotion", "tier2", "midrange", "midrange",
        "Black devotion permanents draining out with Gray Merchant",
        [
            (4, "Thoughtseize"), (4, "Fatal Push"), (4, "Gifted Aetherborn"),
            (4, "Nighthawk Scavenger"), (3, "Ayara, First of Locthwain"), (4, "Murderous Rider"),
            (3, "Phyrexian Arena"), (4, "Invoke Despair"), (4, "Gray Merchant of Asphodel"),
            (2, "Sheoldred, the Apocalypse"), (4, "Castle Locthwain"), (20, "Swamp"),
        ],
        [(4, "Duress"), (4, "Go Blank"), (4, "Unlicensed Hearse"), (3, "Feed the Swarm")],
        {"roles": {"Phyrexian Arena": ["engine"], "Gray Merchant of Asphodel": ["finisher"], "Sheoldred, the Apocalypse": ["finisher", "engine"]}},
    ),
    (
        "pioneer", "tier2-simic-flash", "Simic Flash", "tier2", "tempo", "tempo",
        "Plays almost entirely on the opponent's turn with Nightpack Ambusher",
        [
            (4, "Spectral Sailor"), (4, "Brineborn Cutthroat"), (4, "Wildborn Preserver"),
            (4, "Brazen Borrower"), (4, "Frilled Mystic"), (4, "Nightpack Ambusher"),
            (4, "Growth Spiral"), (4, "Quench"), (4, "Sinister Sabotage"),
            (2, "Shark Typhoon"), (4, "Breeding Pool"), (4, "Botanical Sanctum"),
            (4, "Willowrush Verge"), (6, "Island"), (4, "Forest"),
        ],
        [(4, "Mystical Dispute"), (4, "Aether Gust"), (4, "Pick Your Poison"), (3, "Unlicensed Hearse")],
        {"roles": {"Nightpack Ambusher": ["eot_instant", "finisher"], "Frilled Mystic": ["counter", "eot_instant"], "Sinister Sabotage": ["counter"], "Quench": ["counter"]}},
    ),
    (
        "pioneer", "tier2-izzet-control", "Izzet Control", "tier2", "control", "control",
        "Efficient burn removal, counters, and Shark Typhoon",
        [
            (4, "Consider"), (4, "Opt"), (4, "Expressive Iteration"),
            (4, "Play with Fire"), (3, "Abrade"), (3, "Anger of the Gods"),
            (4, "Sinister Sabotage"), (3, "Memory Deluge"), (3, "Shark Typhoon"),
            (3, "Bonecrusher Giant"), (2, "Narset, Parter of Veils"),
            (4, "Steam Vents"), (4, "Spirebluff Canal"), (4, "Riverpyre Verge"),
            (6, "Island"), (5, "Mountain"),
        ],
        [(4, "Mystical Dispute"), (4, "Aether Gust"), (3, "Fry"), (4, "Unlicensed Hearse")],
        {"roles": {"Sinister Sabotage": ["counter"], "Anger of the Gods": ["sweeper"], "Memory Deluge": ["draw", "eot_instant"], "Shark Typhoon": ["finisher", "eot_instant"]}},
    ),
    (
        "pioneer", "tier2-orzhov-humans", "Orzhov Humans", "tier2", "tribal", "tribal",
        "Taxing white Humans backed by Thoughtseize and Fatal Push",
        [
            (4, "Hopeful Initiate"), (4, "Thalia, Guardian of Thraben"), (4, "Thalia's Lieutenant"),
            (4, "Deep-Cavern Bat"), (4, "Brutal Cathar"), (4, "Adeline, Resplendent Cathar"),
            (3, "Anointed Peacekeeper"), (4, "Thoughtseize"), (4, "Fatal Push"),
            (3, "Portable Hole"), (4, "Godless Shrine"), (4, "Concealed Courtyard"),
            (4, "Bleachbone Verge"), (2, "Mutavault"), (5, "Plains"), (3, "Swamp"),
        ],
        [(3, "Duress"), (3, "Rest in Peace"), (3, "Get Lost"), (3, "Go Blank"), (3, "Authority of the Consuls")],
        {"roles": {"Thalia, Guardian of Thraben": ["hate_piece", "early_play"], "Thalia's Lieutenant": ["lord", "payoff"], "Adeline, Resplendent Cathar": ["finisher"]}},
    ),
    (
        "pioneer", "tier2-golgari-elves", "Golgari Elves", "tier2", "tribal", "tribal",
        "Collected Company Elves finishing with Shaman of the Pack",
        [
            (4, "Llanowar Elves"), (4, "Elvish Mystic"), (4, "Elvish Clancaller"),
            (4, "Elvish Warmaster"), (4, "Leaf-Crowned Visionary"), (4, "Elvish Archdruid"),
            (4, "Shaman of the Pack"), (4, "Collected Company"), (4, "Chord of Calling"),
            (4, "Fatal Push"), (4, "Overgrown Tomb"), (4, "Blooming Marsh"),
            (4, "Wastewood Verge"), (6, "Forest"), (2, "Swamp"),
        ],
        [(4, "Thoughtseize"), (4, "Keen-Eyed Curator"), (4, "Pick Your Poison"), (3, "Abrupt Decay")],
        {"roles": {"Llanowar Elves": ["mana_dork", "early_play"], "Elvish Mystic": ["mana_dork", "early_play"], "Shaman of the Pack": ["finisher"], "Collected Company": ["eot_instant", "payoff"]}},
    ),
    (
        "pioneer", "tier2-abzan-midrange", "Abzan Midrange", "tier2", "midrange", "midrange",
        "Three-color attrition with Vanishing Verse, Glissa, and Sheoldred",
        [
            (4, "Thoughtseize"), (4, "Fatal Push"), (4, "Vanishing Verse"),
            (3, "Abrupt Decay"), (4, "Skyclave Apparition"), (4, "Glissa Sunslayer"),
            (4, "Sentinel of the Nameless City"), (3, "Sheoldred, the Apocalypse"), (3, "Beza, the Bounding Spring"),
            (3, "Deep-Cavern Bat"), (4, "Temple Garden"), (4, "Overgrown Tomb"),
            (4, "Godless Shrine"), (4, "Blooming Marsh"), (4, "Concealed Courtyard"),
            (2, "Plains"), (1, "Swamp"), (1, "Forest"),
        ],
        [(3, "Duress"), (3, "Rest in Peace"), (3, "Knight of Autumn"), (3, "Go Blank"), (3, "Unlicensed Hearse")],
        {"roles": {"Glissa Sunslayer": ["engine", "payoff"], "Sheoldred, the Apocalypse": ["finisher", "engine"]}},
    ),
    (
        "pioneer", "rogue-dimir-mill", "Dimir Mill", "rogue", "combo", "combo",
        "Ruin Crab and Tasha's Hideous Laughter library attack",
        [
            (4, "Ruin Crab"), (4, "Maddening Cacophony"), (4, "Tasha's Hideous Laughter"),
            (4, "Jace, the Perfected Mind"), (4, "Drown in the Loch"), (4, "Fatal Push"),
            (4, "Into the Story"), (4, "Consider"), (4, "Cling to Dust"),
            (4, "Fabled Passage"), (4, "Watery Grave"), (4, "Darkslick Shores"),
            (4, "Gloomlake Verge"), (5, "Island"), (3, "Swamp"),
        ],
        [(4, "Thoughtseize"), (4, "Mystical Dispute"), (4, "Aether Gust"), (3, "Unlicensed Hearse")],
        {"roles": {"Ruin Crab": ["engine", "early_play"], "Drown in the Loch": ["counter", "cheap_removal"], "Jace, the Perfected Mind": ["finisher"]}},
    ),
    (
        "pioneer", "rogue-izzet-ensoul", "Izzet Ensoul Artifact", "rogue", "aggro", "aggro",
        "5/5 Darksteel Citadels and Shrapnel Blast reach",
        [
            (4, "Ornithopter"), (4, "Gingerbrute"), (4, "Springleaf Drum"),
            (4, "Ensoul Artifact"), (4, "Patchwork Automaton"), (4, "Steel Overseer"),
            (4, "Shrapnel Blast"), (4, "Stubborn Denial"), (4, "Play with Fire"),
            (2, "Shadowspear"), (4, "Darksteel Citadel"), (4, "Steam Vents"),
            (4, "Spirebluff Canal"), (4, "Riverpyre Verge"), (3, "Island"), (3, "Mountain"),
        ],
        [(4, "Abrade"), (4, "Mystical Dispute"), (4, "Unlicensed Hearse"), (3, "Fry")],
        {"roles": {"Ensoul Artifact": ["payoff"], "Shrapnel Blast": ["burn"], "Stubborn Denial": ["counter"]}},
    ),
    (
        "pioneer", "rogue-mono-white-lifegain", "Mono-White Lifegain", "rogue", "tribal", "tribal",
        "Soul sisters fueling Voice of the Blessed and Angels",
        [
            (4, "Lunarch Veteran"), (4, "Authority of the Consuls"), (4, "Ajani's Pridemate"),
            (4, "Voice of the Blessed"), (4, "Bishop of Wings"), (4, "Righteous Valkyrie"),
            (4, "Resplendent Angel"), (4, "Skyclave Apparition"), (4, "Portable Hole"),
            (2, "Lyra Dawnbringer"), (22, "Plains"),
        ],
        [(4, "Rest in Peace"), (4, "Get Lost"), (4, "Damping Sphere"), (3, "Day of Judgment")],
        {"roles": {"Bishop of Wings": ["engine"], "Righteous Valkyrie": ["lord", "payoff"]}},
    ),

    # ===================== MODERN (+19 -> 30 total) =====================
    (
        "modern", "tier1-azorius-control", "Azorius Control", "meta", "control", "control",
        "Teferi, Solitude, and Force of Negation control shell",
        [
            (4, "Path to Exile"), (4, "Prismatic Ending"), (3, "March of Otherworldly Light"),
            (4, "Force of Negation"), (3, "Spell Snare"), (2, "Dovin's Veto"),
            (3, "Supreme Verdict"), (4, "Teferi, Time Raveler"), (2, "Teferi, Hero of Dominaria"),
            (2, "The Wandering Emperor"), (4, "Solitude"), (2, "Shark Typhoon"),
            (4, "Hallowed Fountain"), (4, "Seachrome Coast"), (4, "Floodfarm Verge"),
            (2, "Demolition Field"), (5, "Island"), (4, "Plains"),
        ],
        [(2, "Dovin's Veto"), (3, "Rest in Peace"), (3, "Mystical Dispute"), (3, "Wear // Tear"), (1, "Supreme Verdict"), (3, "Portable Hole")],
        {
            "keyCards": ["Teferi, Time Raveler", "Solitude", "Force of Negation", "Supreme Verdict"],
            "roles": {
                "Force of Negation": ["counter"], "Spell Snare": ["counter"], "Dovin's Veto": ["counter"],
                "Supreme Verdict": ["sweeper"], "The Wandering Emperor": ["eot_instant", "cheap_removal", "engine"],
                "Teferi, Time Raveler": ["engine"], "Teferi, Hero of Dominaria": ["finisher", "engine"],
                "Solitude": ["cheap_removal", "eot_instant"], "Shark Typhoon": ["finisher", "eot_instant"],
            },
            "cardRules": [
                {"card": "The Wandering Emperor", "holdForStep": "combat_attackers", "unless": {"and": [{"notMyTurn": True}, {"step": "EndTurn"}]}},
                {"card": "Supreme Verdict", "castOnlyIf": {"or": [{"netOppCreaturesAtLeast": 1}, {"oppPowerAtLeast": 5}, {"myLifeAtMost": 10}]}},
            ],
            "sideboard": {
                "vs_aggro": {"in": {"Supreme Verdict": 1, "Portable Hole": 3}, "out": {"Force of Negation": 2, "Dovin's Veto": 2}},
                "vs_control": {"in": {"Dovin's Veto": 2, "Mystical Dispute": 3}, "out": {"Supreme Verdict": 3, "Path to Exile": 2}},
                "vs_graveyard": {"in": {"Rest in Peace": 3}, "out": {"Spell Snare": 3}},
            },
        },
    ),
    (
        "modern", "tier1-jund-midrange", "Jund Midrange", "meta", "midrange", "midrange",
        "Classic Tarmogoyf, Liliana of the Veil, and Kolaghan's Command attrition",
        [
            (4, "Thoughtseize"), (4, "Fatal Push"), (4, "Lightning Bolt"),
            (3, "Abrupt Decay"), (2, "Assassin's Trophy"), (4, "Tarmogoyf"),
            (4, "Dark Confidant"), (3, "Scavenging Ooze"), (3, "Liliana of the Veil"),
            (3, "Kolaghan's Command"), (3, "Fable of the Mirror-Breaker"), (2, "Kroxa, Titan of Death's Hunger"),
            (4, "Blood Crypt"), (4, "Overgrown Tomb"), (3, "Stomping Ground"),
            (4, "Blackcleave Cliffs"), (4, "Blooming Marsh"), (1, "Swamp"), (1, "Forest"),
        ],
        [(3, "Duress"), (3, "Leyline of the Void"), (3, "Abrade"), (3, "Anger of the Gods"), (3, "Pick Your Poison")],
        {
            "keyCards": ["Tarmogoyf", "Liliana of the Veil", "Kolaghan's Command", "Dark Confidant"],
            "roles": {
                "Thoughtseize": ["early_play"], "Dark Confidant": ["engine", "early_play"],
                "Tarmogoyf": ["payoff"], "Liliana of the Veil": ["engine"],
                "Fable of the Mirror-Breaker": ["engine", "payoff"], "Lightning Bolt": ["burn", "cheap_removal"],
            },
            "playOrder": ["early_play", "cheap_removal", "engine", "payoff"],
            "evalCardBonus": {"Liliana of the Veil": 0.55, "Fable of the Mirror-Breaker": 0.55},
            "sideboard": {
                "vs_aggro": {"in": {"Anger of the Gods": 3, "Abrade": 2}, "out": {"Thoughtseize": 3, "Dark Confidant": 2}},
                "vs_control": {"in": {"Duress": 3}, "out": {"Fatal Push": 3}},
                "vs_graveyard": {"in": {"Leyline of the Void": 3}, "out": {"Abrupt Decay": 3}},
            },
        },
    ),
    (
        "modern", "tier1-dimir-tempo", "Dimir Frog Tempo", "meta", "tempo", "tempo",
        "Psychic Frog card advantage protected by Force of Negation",
        [
            (4, "Psychic Frog"), (4, "Faerie Mastermind"), (4, "Brazen Borrower"),
            (3, "Sheoldred, the Apocalypse"), (4, "Fatal Push"), (4, "Thoughtseize"),
            (4, "Drown in the Loch"), (4, "Force of Negation"), (3, "Spell Snare"),
            (4, "Consider"), (2, "Opt"), (4, "Watery Grave"),
            (4, "Darkslick Shores"), (4, "Gloomlake Verge"), (5, "Island"), (3, "Swamp"),
        ],
        [(3, "Mystical Dispute"), (3, "Duress"), (3, "Leyline of the Void"), (3, "Unlicensed Hearse"), (3, "Go Blank")],
        {
            "keyCards": ["Psychic Frog", "Force of Negation", "Drown in the Loch", "Fatal Push"],
            "roles": {
                "Psychic Frog": ["early_play", "engine", "payoff"], "Faerie Mastermind": ["early_play", "eot_instant"],
                "Force of Negation": ["counter"], "Spell Snare": ["counter"], "Drown in the Loch": ["counter", "cheap_removal"],
                "Fatal Push": ["cheap_removal", "eot_instant"],
            },
            "evalCardBonus": {"Psychic Frog": 0.65},
            "sideboard": {
                "vs_control": {"in": {"Mystical Dispute": 3, "Duress": 3}, "out": {"Fatal Push": 4, "Spell Snare": 2}},
                "vs_graveyard": {"in": {"Leyline of the Void": 3, "Unlicensed Hearse": 2}, "out": {"Spell Snare": 3, "Opt": 2}},
            },
        },
    ),
    (
        "modern", "tier1-selesnya-company", "Selesnya Hatebears", "meta", "midrange", "midrange",
        "Taxing creatures and Collected Company value",
        [
            (4, "Llanowar Elves"), (4, "Elvish Mystic"), (4, "Thalia, Guardian of Thraben"),
            (4, "Archon of Emeria"), (4, "Skyclave Apparition"), (3, "Knight of Autumn"),
            (3, "Scavenging Ooze"), (3, "Tireless Tracker"), (4, "Collected Company"),
            (4, "Path to Exile"), (2, "Selfless Spirit"), (4, "Temple Garden"),
            (4, "Razorverge Thicket"), (4, "Brushland"), (4, "Hushwood Verge"),
            (3, "Plains"), (2, "Forest"),
        ],
        [(3, "Rest in Peace"), (3, "Damping Sphere"), (3, "Portable Hole"), (3, "Pick Your Poison"), (3, "Get Lost")],
        {
            "keyCards": ["Collected Company", "Archon of Emeria", "Skyclave Apparition", "Thalia, Guardian of Thraben"],
            "roles": {"Llanowar Elves": ["mana_dork", "early_play"], "Elvish Mystic": ["mana_dork", "early_play"], "Collected Company": ["eot_instant", "payoff"]},
            "cardRules": [{"card": "Collected Company", "holdForStep": "opp_end_step"}],
        },
    ),
    (
        "modern", "tier1-rakdos-sacrifice", "Rakdos Sacrifice", "meta", "sacrifice", "sacrifice",
        "Cauldron Familiar, Witch's Oven, and Mayhem Devil burn",
        [
            (4, "Cauldron Familiar"), (4, "Witch's Oven"), (4, "Mayhem Devil"),
            (4, "Voldaren Epicure"), (4, "Deadly Dispute"), (4, "Village Rites"),
            (4, "Fatal Push"), (4, "Lightning Bolt"), (3, "Thoughtseize"),
            (3, "Fable of the Mirror-Breaker"), (2, "Kroxa, Titan of Death's Hunger"),
            (4, "Blood Crypt"), (4, "Blackcleave Cliffs"), (4, "Blazemire Verge"),
            (5, "Swamp"), (3, "Mountain"),
        ],
        [(3, "Duress"), (3, "Leyline of the Void"), (3, "Abrade"), (3, "Kolaghan's Command"), (3, "Anger of the Gods")],
        {"roles": {"Cauldron Familiar": ["fodder", "early_play"], "Witch's Oven": ["sac_outlet", "early_play"], "Mayhem Devil": ["payoff", "engine"]}},
    ),
    (
        "modern", "tier1-esper-control", "Esper Blink Control", "meta", "control", "control",
        "Solitude + Ephemerate loops behind Esper countermagic",
        [
            (4, "Solitude"), (4, "Ephemerate"), (4, "Fatal Push"),
            (4, "Path to Exile"), (3, "Prismatic Ending"), (4, "Force of Negation"),
            (2, "Dovin's Veto"), (3, "Teferi, Time Raveler"), (3, "Supreme Verdict"),
            (3, "Memory Deluge"), (2, "Shark Typhoon"), (4, "Hallowed Fountain"),
            (4, "Watery Grave"), (4, "Godless Shrine"), (4, "Floodfarm Verge"),
            (4, "Gloomlake Verge"), (2, "Island"), (1, "Plains"), (1, "Swamp"),
        ],
        [(2, "Dovin's Veto"), (3, "Rest in Peace"), (3, "Thoughtseize"), (3, "Mystical Dispute"), (4, "Wear // Tear")],
        {
            "keyCards": ["Solitude", "Ephemerate", "Teferi, Time Raveler", "Supreme Verdict"],
            "roles": {"Force of Negation": ["counter"], "Dovin's Veto": ["counter"], "Supreme Verdict": ["sweeper"], "Memory Deluge": ["draw", "eot_instant"]},
            "cardRules": [{"card": "Memory Deluge", "holdForStep": "opp_end_step"}],
        },
    ),
    (
        "modern", "tier1-jeskai-control", "Jeskai Control", "meta", "control", "control",
        "Bolt-Helix control with Expressive Iteration and Teferi",
        [
            (4, "Lightning Bolt"), (4, "Lightning Helix"), (4, "Path to Exile"),
            (3, "Prismatic Ending"), (4, "Force of Negation"), (3, "Spell Snare"),
            (4, "Expressive Iteration"), (4, "Teferi, Time Raveler"), (3, "Supreme Verdict"),
            (2, "The Wandering Emperor"), (2, "Shark Typhoon"), (4, "Hallowed Fountain"),
            (4, "Steam Vents"), (4, "Sacred Foundry"), (4, "Floodfarm Verge"),
            (3, "Riverpyre Verge"), (2, "Island"), (1, "Plains"), (1, "Mountain"),
        ],
        [(3, "Wear // Tear"), (3, "Rest in Peace"), (3, "Dovin's Veto"), (3, "Mystical Dispute"), (3, "Abrade")],
        {
            "roles": {"Force of Negation": ["counter"], "Spell Snare": ["counter"], "Supreme Verdict": ["sweeper"], "The Wandering Emperor": ["eot_instant", "engine"]},
            "cardRules": [{"card": "The Wandering Emperor", "holdForStep": "combat_attackers"}],
        },
    ),
    (
        "modern", "tier1-boros-aggro", "Boros Ragavan Aggro", "meta", "aggro", "aggro",
        "Ragavan and hasty beaters backed by Lightning Bolt and Boros Charm",
        [
            (4, "Ragavan, Nimble Pilferer"), (4, "Goblin Guide"), (4, "Monastery Swiftspear"),
            (4, "Slickshot Show-Off"), (4, "Eidolon of the Great Revel"), (4, "Lightning Bolt"),
            (4, "Lightning Helix"), (4, "Boros Charm"), (4, "Path to Exile"),
            (4, "Lava Spike"), (4, "Sacred Foundry"), (4, "Inspiring Vantage"),
            (4, "Sunbillow Verge"), (5, "Mountain"), (3, "Plains"),
        ],
        [(3, "Wear // Tear"), (3, "Rest in Peace"), (3, "Searing Blaze"), (3, "Deflecting Palm"), (3, "Portable Hole")],
        {"roles": {"Ragavan, Nimble Pilferer": ["early_play", "engine"], "Goblin Guide": ["early_play", "haste"], "Lightning Bolt": ["burn"]}},
    ),
    (
        "modern", "tier1-izzet-phoenix", "Izzet Phoenix", "meta", "graveyard", "graveyard",
        "Cantrip chains resurrecting Arclight Phoenixes",
        [
            (4, "Arclight Phoenix"), (4, "Dragon's Rage Channeler"), (4, "Ledger Shredder"),
            (4, "Monastery Swiftspear"), (4, "Lightning Bolt"), (4, "Consider"),
            (4, "Opt"), (4, "Sleight of Hand"), (4, "Expressive Iteration"),
            (4, "Chart a Course"), (4, "Steam Vents"), (4, "Spirebluff Canal"),
            (4, "Riverpyre Verge"), (5, "Island"), (3, "Mountain"),
        ],
        [(3, "Abrade"), (3, "Mystical Dispute"), (3, "Force of Negation"), (3, "Unlicensed Hearse"), (3, "Anger of the Gods")],
        {
            "keyCards": ["Arclight Phoenix", "Dragon's Rage Channeler", "Ledger Shredder", "Expressive Iteration"],
            "roles": {"Arclight Phoenix": ["reanimate_target", "payoff"], "Ledger Shredder": ["engine", "enabler"], "Chart a Course": ["enabler", "draw"]},
            "discardRolesFirst": ["reanimate_target"],
            "graveyardRoleBonus": {"reanimate_target": 0.50},
        },
    ),
    (
        "modern", "tier2-mardu-pyromancer", "Mardu Pyromancer", "tier2", "midrange", "midrange",
        "Spell-token engines with Young Pyromancer and Monastery Mentor",
        [
            (4, "Young Pyromancer"), (3, "Monastery Mentor"), (3, "Third Path Iconoclast"),
            (4, "Thoughtseize"), (4, "Fatal Push"), (4, "Lightning Bolt"),
            (4, "Lightning Helix"), (3, "Kolaghan's Command"), (3, "Dreadbore"),
            (4, "Fable of the Mirror-Breaker"), (2, "Path to Exile"),
            (4, "Blood Crypt"), (4, "Sacred Foundry"), (4, "Godless Shrine"),
            (4, "Blackcleave Cliffs"), (4, "Inspiring Vantage"), (1, "Swamp"), (1, "Mountain"),
        ],
        [(3, "Wear // Tear"), (3, "Rest in Peace"), (3, "Duress"), (3, "Anger of the Gods"), (3, "Unlicensed Hearse")],
        {"roles": {"Young Pyromancer": ["engine"], "Monastery Mentor": ["engine", "finisher"], "Fable of the Mirror-Breaker": ["engine"]}},
    ),
    (
        "modern", "tier2-abzan-midrange", "Abzan Midrange", "tier2", "midrange", "midrange",
        "Tarmogoyf, Solitude, and Liliana grind",
        [
            (4, "Thoughtseize"), (4, "Fatal Push"), (4, "Path to Exile"),
            (3, "Abrupt Decay"), (3, "Vanishing Verse"), (4, "Tarmogoyf"),
            (4, "Dark Confidant"), (3, "Scavenging Ooze"), (3, "Liliana of the Veil"),
            (3, "Solitude"), (3, "Skyclave Apparition"), (4, "Overgrown Tomb"),
            (4, "Godless Shrine"), (4, "Temple Garden"), (4, "Blooming Marsh"),
            (4, "Concealed Courtyard"), (1, "Swamp"), (1, "Plains"),
        ],
        [(3, "Rest in Peace"), (3, "Duress"), (3, "Knight of Autumn"), (3, "Leyline of the Void"), (3, "Portable Hole")],
        {"roles": {"Dark Confidant": ["engine"], "Liliana of the Veil": ["engine"], "Tarmogoyf": ["payoff"]}},
    ),
    (
        "modern", "tier2-mono-white-taxes", "Mono-White Taxes", "tier2", "tribal", "tribal",
        "Thalia taxes, Solitude-Ephemerate, and white pressure",
        [
            (4, "Thalia, Guardian of Thraben"), (3, "Thalia, Heretic Cathar"), (4, "Skyclave Apparition"),
            (4, "Adeline, Resplendent Cathar"), (3, "Brimaz, King of Oreskos"), (4, "Solitude"),
            (4, "Ephemerate"), (4, "Path to Exile"), (4, "Portable Hole"),
            (4, "Aether Vial"), (2, "Mutavault"), (20, "Plains"),
        ],
        [(4, "Rest in Peace"), (4, "Damping Sphere"), (4, "Get Lost"), (3, "Authority of the Consuls")],
        {"roles": {"Thalia, Guardian of Thraben": ["hate_piece", "early_play"], "Aether Vial": ["engine", "early_play"], "Solitude": ["cheap_removal", "eot_instant"]}},
    ),
    (
        "modern", "tier2-bant-spirits", "Bant Spirits", "tier2", "tempo", "tempo",
        "Mana elves accelerating Spell Queller and Collected Company Spirits",
        [
            (4, "Llanowar Elves"), (4, "Mausoleum Wanderer"), (4, "Rattlechains"),
            (4, "Supreme Phantom"), (4, "Spell Queller"), (3, "Selfless Spirit"),
            (3, "Shacklegeist"), (4, "Collected Company"), (4, "Path to Exile"),
            (2, "Force of Negation"), (4, "Hallowed Fountain"), (4, "Breeding Pool"),
            (4, "Temple Garden"), (4, "Seachrome Coast"), (4, "Botanical Sanctum"),
            (2, "Island"), (2, "Plains"),
        ],
        [(2, "Force of Negation"), (3, "Rest in Peace"), (3, "Dovin's Veto"), (4, "Knight of Autumn"), (3, "Portable Hole")],
        {"roles": {"Llanowar Elves": ["mana_dork", "early_play"], "Spell Queller": ["counter", "eot_instant"], "Collected Company": ["eot_instant", "payoff"]}, "cardRules": [{"card": "Collected Company", "holdForStep": "opp_end_step"}]},
    ),
    (
        "modern", "tier2-golgari-elves", "Golgari Elves", "tier2", "tribal", "tribal",
        "Heritage Druid mana bursts into Ezuri and Shaman of the Pack",
        [
            (4, "Llanowar Elves"), (4, "Elvish Mystic"), (4, "Heritage Druid"),
            (4, "Nettle Sentinel"), (4, "Elvish Clancaller"), (4, "Elvish Warmaster"),
            (4, "Ezuri, Renegade Leader"), (4, "Shaman of the Pack"), (4, "Collected Company"),
            (4, "Chord of Calling"), (4, "Overgrown Tomb"), (4, "Blooming Marsh"),
            (4, "Wastewood Verge"), (6, "Forest"), (2, "Swamp"),
        ],
        [(4, "Thoughtseize"), (4, "Fatal Push"), (4, "Scavenging Ooze"), (3, "Pick Your Poison")],
        {"roles": {"Heritage Druid": ["mana_dork", "engine"], "Ezuri, Renegade Leader": ["lord", "finisher"], "Shaman of the Pack": ["finisher"], "Collected Company": ["eot_instant", "payoff"]}},
    ),
    (
        "modern", "tier2-mono-blue-merfolk", "Mono-Blue Merfolk", "tier2", "tribal", "tribal",
        "Aether Vial Merfolk lords and Force of Negation",
        [
            (4, "Aether Vial"), (4, "Lord of Atlantis"), (4, "Master of the Pearl Trident"),
            (4, "Silvergill Adept"), (4, "Vodalian Hexcatcher"), (4, "Svyelun of Sea and Sky"),
            (4, "Merfolk Trickster"), (4, "Harbinger of the Tides"), (4, "Tishana's Tidebinder"),
            (4, "Force of Negation"), (4, "Mutavault"), (16, "Island"),
        ],
        [(4, "Mystical Dispute"), (4, "Unlicensed Hearse"), (4, "Damping Sphere"), (3, "Dismember")],
        {"roles": {"Aether Vial": ["engine", "early_play"], "Lord of Atlantis": ["lord"], "Master of the Pearl Trident": ["lord"], "Force of Negation": ["counter"]}},
    ),
    (
        "modern", "tier2-gruul-midrange", "Gruul Ragavan Midrange", "tier2", "midrange", "midrange",
        "Ragavan and elves ramping into Fable, Chariot, and Questing Beast",
        [
            (4, "Llanowar Elves"), (4, "Elvish Mystic"), (4, "Ragavan, Nimble Pilferer"),
            (4, "Tarmogoyf"), (4, "Bonecrusher Giant"), (4, "Fable of the Mirror-Breaker"),
            (4, "Esika's Chariot"), (3, "Questing Beast"), (4, "Lightning Bolt"),
            (3, "Abrade"), (4, "Stomping Ground"), (4, "Copperline Gorge"),
            (4, "Thornspire Verge"), (6, "Forest"), (4, "Mountain"),
        ],
        [(4, "Pick Your Poison"), (4, "Scavenging Ooze"), (4, "Unlicensed Hearse"), (3, "Anger of the Gods")],
        {"roles": {"Ragavan, Nimble Pilferer": ["early_play", "engine"], "Fable of the Mirror-Breaker": ["engine"], "Esika's Chariot": ["payoff"]}},
    ),
    (
        "modern", "rogue-mono-red-goblins", "Mono-Red Goblins", "rogue", "tribal", "tribal",
        "Aether Vial Goblins with Conspicuous Snoop and Krenko",
        [
            (4, "Aether Vial"), (4, "Skirk Prospector"), (4, "Conspicuous Snoop"),
            (4, "Goblin Warchief"), (4, "Goblin Piledriver"), (4, "Goblin Rabblemaster"),
            (4, "Rundvelt Hordemaster"), (3, "Krenko, Mob Boss"), (3, "Legion Warboss"),
            (4, "Lightning Bolt"), (2, "Mutavault"), (20, "Mountain"),
        ],
        [(4, "Abrade"), (4, "Unlicensed Hearse"), (4, "Damping Sphere"), (3, "Searing Blaze")],
        {"roles": {"Aether Vial": ["engine"], "Goblin Warchief": ["lord"], "Krenko, Mob Boss": ["finisher"]}},
    ),
    (
        "modern", "rogue-dimir-mill", "Dimir Mill", "rogue", "combo", "combo",
        "Ruin Crab and Tasha's Hideous Laughter backed by Force of Negation",
        [
            (4, "Ruin Crab"), (4, "Maddening Cacophony"), (4, "Tasha's Hideous Laughter"),
            (4, "Jace, the Perfected Mind"), (4, "Drown in the Loch"), (4, "Fatal Push"),
            (4, "Force of Negation"), (4, "Into the Story"), (4, "Consider"),
            (4, "Fabled Passage"), (4, "Watery Grave"), (4, "Darkslick Shores"),
            (4, "Gloomlake Verge"), (5, "Island"), (3, "Swamp"),
        ],
        [(4, "Thoughtseize"), (4, "Leyline of the Void"), (4, "Mystical Dispute"), (3, "Unlicensed Hearse")],
        {"roles": {"Ruin Crab": ["engine"], "Force of Negation": ["counter"], "Drown in the Loch": ["counter", "cheap_removal"]}},
    ),
    (
        "modern", "rogue-mono-black-midrange", "Mono-Black Midrange", "rogue", "midrange", "midrange",
        "Liliana, Dark Confidant, Sheoldred, and Gray Merchant drain",
        [
            (4, "Thoughtseize"), (4, "Fatal Push"), (4, "Gifted Aetherborn"),
            (4, "Dark Confidant"), (4, "Liliana of the Veil"), (4, "Nighthawk Scavenger"),
            (3, "Phyrexian Arena"), (4, "Sheoldred, the Apocalypse"), (3, "Gray Merchant of Asphodel"),
            (2, "Go for the Throat"), (4, "Castle Locthwain"), (20, "Swamp"),
        ],
        [(4, "Duress"), (4, "Leyline of the Void"), (4, "Unlicensed Hearse"), (3, "Feed the Swarm")],
        {"roles": {"Dark Confidant": ["engine"], "Liliana of the Veil": ["engine"], "Sheoldred, the Apocalypse": ["finisher"]}},
    ),
]


def main():
    verified = {line.strip() for line in open(VERIFIED_PATH, encoding="utf-8") if line.strip()}
    opener = gzip.open if VIEWS_PATH.endswith(".gz") else open
    with opener(VIEWS_PATH, "rt", encoding="utf-8") as f:
        views = json.load(f)

    # Fix any missing/unverified cards early with a clear report
    missing = []
    for fmt, slug, name, tier, arch, profile, tagline, main_cards, sb_cards, _ in DECKS:
        for _, card in main_cards + sb_cards:
            if card not in verified:
                missing.append((fmt, slug, card, "not in verified.txt"))
            elif card not in BASIC_PRINTINGS and card not in views:
                missing.append((fmt, slug, card, "not in views.json"))
    if missing:
        for m in missing:
            print("MISSING:", m)
        sys.exit(1)

    playbooks = json.load(open(RUST_PLAYBOOKS, encoding="utf-8"))
    manifest = {}

    for fmt, slug, name, tier, arch, profile, tagline, main_cards, sb_cards, extra in DECKS:
        main_total = sum(c for c, _ in main_cards)
        sb_total = sum(c for c, _ in sb_cards)
        assert main_total == 60, f"{fmt}/{slug} main={main_total}"
        assert sb_total == 15, f"{fmt}/{slug} sb={sb_total}"

        out_dir = os.path.join(LADDER_DECKS, fmt)
        os.makedirs(out_dir, exist_ok=True)
        out_path = os.path.join(out_dir, f"{slug}.dck")
        lines = [f"NAME:{name}"]
        for count, card in main_cards:
            if card in BASIC_PRINTINGS:
                code, num = BASIC_PRINTINGS[card]
            else:
                v = views[card]
                code, num = v["expansionSetCode"], v["cardNumber"]
            lines.append(f"{count} [{code}:{num}] {card}")
        for count, card in sb_cards:
            if card in BASIC_PRINTINGS:
                code, num = BASIC_PRINTINGS[card]
            else:
                v = views[card]
                code, num = v["expansionSetCode"], v["cardNumber"]
            lines.append(f"SB: {count} [{code}:{num}] {card}")
        with open(out_path, "w", encoding="utf-8") as f:
            f.write("\n".join(lines) + "\n")

        pb_id = f"ladder/{fmt}/{slug}"
        pb = {
            "profile": profile,
            "archetype": arch,
            "tier": tier,
            "tagline": tagline,
        }
        pb.update(extra)
        playbooks["decks"][pb_id] = pb
        manifest[pb_id] = {"archetype": arch, "tier": tier, "tagline": tagline, "keyCards": extra.get("keyCards", [])}

    for path in (RUST_PLAYBOOKS, JAVA_PLAYBOOKS):
        with open(path, "w", encoding="utf-8") as f:
            json.dump(playbooks, f, indent=2, ensure_ascii=False)
            f.write("\n")

    print(f"Wrote {len(DECKS)} ladder decks and synced {len(playbooks['decks'])} deck playbooks to Rust and Java.")


if __name__ == "__main__":
    main()
