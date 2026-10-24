import type { CardView } from '../../protocol/generated/views';
import { stripMarkup } from './prompt';

/**
 * Plain-language help for what a card's text does: which kind of ability each line is, and what the keywords and
 * game actions it mentions mean. Explanations are written for this client, short enough to read mid-game.
 */

export interface Term {
  name: string;
  text: string;
}

interface Entry extends Term {
  /** also matches these forms (lowercase), e.g. "fights" */
  forms?: string[];
  /**
   * Only recognized as a keyword at the start of a line or of a comma-separated keyword list: the word also shows up
   * in card names and ordinary sentences.
   */
  strict?: boolean;
  /** overrides the name-and-forms match */
  match?: RegExp;
  /** the counter this explains ("stun"): also listed for a permanent that has one, whatever its text says */
  counter?: string;
}

const KEYWORDS: Entry[] = [
  { name: 'Flying', text: "Can't be blocked except by creatures with flying or reach." },
  { name: 'Reach', text: 'Can block creatures with flying.' },
  { name: 'Vigilance', text: "Attacking doesn't cause it to tap." },
  { name: 'Trample', text: "Combat damage beyond what's needed to destroy its blockers is dealt to the player or planeswalker it's attacking." },
  { name: 'Haste', text: 'Can attack and use {T} abilities the turn it comes under your control.' },
  { name: 'Deathtouch', text: 'Any amount of damage it deals to a creature is enough to destroy that creature.' },
  { name: 'Lifelink', text: 'Damage it deals also makes its controller gain that much life.' },
  { name: 'First strike', text: 'Deals combat damage before creatures without first strike.' },
  { name: 'Double strike', text: 'Deals combat damage twice: once with first strike, then again with the regular damage.' },
  { name: 'Menace', text: "Can't be blocked except by two or more creatures." },
  { name: 'Defender', text: "Can't attack." },
  { name: 'Hexproof', text: "Can't be the target of spells or abilities your opponents control." },
  { name: 'Shroud', text: "Can't be the target of any spells or abilities." },
  { name: 'Indestructible', text: "Damage and effects that say \"destroy\" don't destroy it." },
  { name: 'Flash', text: 'Can be cast any time you could cast an instant, even on an opponent\'s turn.' },
  { name: 'Ward', text: "When it becomes the target of a spell or ability an opponent controls, that spell or ability is countered unless its controller pays the ward cost." },
  { name: 'Prowess', text: 'Whenever you cast a noncreature spell, it gets +1/+1 until end of turn.' },
  { name: 'Protection', text: "Can't be blocked, targeted, dealt damage, enchanted or equipped by anything with the stated quality." },
  { name: 'Fear', text: "Can't be blocked except by artifact creatures and black creatures.", strict: true },
  { name: 'Intimidate', text: "Can't be blocked except by artifact creatures and creatures that share a color with it." },
  { name: 'Skulk', text: "Can't be blocked by creatures with greater power." },
  { name: 'Shadow', text: 'Can block or be blocked only by creatures with shadow.', strict: true },
  { name: 'Landwalk', text: "Can't be blocked as long as the defending player controls a land of the stated type.", forms: ['plainswalk', 'islandwalk', 'swampwalk', 'mountainwalk', 'forestwalk'] },
  { name: 'Infect', text: 'Deals damage to creatures as -1/-1 counters and to players as poison counters.' },
  { name: 'Wither', text: 'Deals damage to creatures as -1/-1 counters.' },
  { name: 'Toxic', text: 'Combat damage it deals to a player also gives them that many poison counters.' },
  { name: 'Poison', text: 'A player with ten or more poison counters loses the game.', forms: ['poison counter', 'poison counters'] },
  { name: 'Annihilator', text: 'Whenever it attacks, the defending player sacrifices that many permanents.' },
  { name: 'Exalted', text: 'Whenever a creature you control attacks alone, it gets +1/+1 until end of turn for each instance of exalted you control.' },
  { name: 'Persist', text: 'When it dies without a -1/-1 counter on it, it returns to the battlefield with a -1/-1 counter.' },
  { name: 'Undying', text: 'When it dies without a +1/+1 counter on it, it returns to the battlefield with a +1/+1 counter.' },
  { name: 'Evolve', text: 'Whenever a creature with greater power or toughness enters under your control, put a +1/+1 counter on this.' },
  { name: 'Changeling', text: 'It is every creature type at all times.' },
  { name: 'Split second', text: "While it's on the stack, players can't cast spells or activate abilities that aren't mana abilities." },
  { name: 'Phasing', text: "It phases out on its controller's untap step if phased in (and in again if phased out). Phased-out permanents are treated as though they don't exist." },
  { name: 'Riot', text: 'It enters with your choice of a +1/+1 counter or haste.', strict: true },
  { name: 'Afflict', text: 'Whenever it becomes blocked, the defending player loses that much life.' },
  { name: 'Afterlife', text: 'When it dies, create that many 1/1 white and black Spirit tokens with flying.' },
  { name: 'Bushido', text: 'Whenever it blocks or becomes blocked, it gets +N/+N until end of turn.' },
  { name: 'Rampage', text: 'Whenever it becomes blocked, it gets +N/+N for each creature blocking it beyond the first.' },
  { name: 'Flanking', text: 'Whenever a creature without flanking blocks it, the blocker gets -1/-1 until end of turn.' },
  { name: 'Provoke', text: 'When it attacks, you may have target creature the defending player controls untap and block it if able.' },
  { name: 'Dethrone', text: 'Whenever it attacks the player with the most life (or tied for most), put a +1/+1 counter on it.' },
  { name: 'Mentor', text: 'Whenever it attacks, put a +1/+1 counter on target attacking creature with lesser power.', strict: true },
  { name: 'Training', text: 'Whenever it attacks with another creature with greater power, put a +1/+1 counter on it.', strict: true },
  { name: 'Renown', text: "When it deals combat damage to a player, if it isn't renowned, put that many +1/+1 counters on it and it becomes renowned." },
  { name: 'Bloodthirst', text: 'If an opponent was dealt damage this turn, it enters with that many +1/+1 counters.' },
  { name: 'Battle cry', text: 'Whenever it attacks, each other attacking creature gets +1/+0 until end of turn.' },
  { name: 'Myriad', text: "Whenever it attacks, for each other opponent, create a token copy attacking that player; exile the tokens at end of combat." },
  { name: 'Melee', text: 'Whenever it attacks, it gets +1/+1 until end of turn for each opponent you attacked this combat.' },
  { name: 'Partner', text: 'You can have two commanders if both have partner.', strict: true },
  { name: 'Soulbond', text: 'You may pair it with another unpaired creature when either enters; they stay paired while you control both.' },
  { name: 'Devour', text: 'As it enters, you may sacrifice any number of creatures; it enters with that many +1/+1 counters for each.' },
  { name: 'Modular', text: 'Enters with that many +1/+1 counters. When it dies, you may move its +1/+1 counters onto target artifact creature.' },
  { name: 'Graft', text: 'Enters with that many +1/+1 counters. Whenever another creature enters, you may move one of them onto it.', strict: true },
  { name: 'Fading', text: 'Enters with that many fade counters. At your upkeep, remove one; if you can\'t, sacrifice it.' },
  { name: 'Vanishing', text: 'Enters with that many time counters. At your upkeep, remove one; when the last is removed, sacrifice it.' },
  { name: 'Echo', text: 'At the beginning of your upkeep after it entered, sacrifice it unless you pay its echo cost.', strict: true },
  { name: 'Cumulative upkeep', text: 'At your upkeep, put an age counter on it, then sacrifice it unless you pay the cost once for each age counter.' },
  { name: 'Outlast', text: 'Pay the cost and {T}: put a +1/+1 counter on it. Only as a sorcery.' },
  { name: 'Unearth', text: 'Pay the cost: return it from your graveyard with haste. Exile it at the next end step or if it would leave. Only as a sorcery.' },
  { name: 'Scavenge', text: 'Pay the cost and exile it from your graveyard: put +1/+1 counters equal to its power on target creature. Only as a sorcery.' },
  { name: 'Embalm', text: 'Pay the cost and exile it from your graveyard: create a white Zombie token copy of it. Only as a sorcery.' },
  { name: 'Eternalize', text: 'Pay the cost and exile it from your graveyard: create a black 4/4 Zombie token copy of it. Only as a sorcery.' },
  { name: 'Encore', text: "Pay the cost and exile it from your graveyard: for each opponent, create a token copy that attacks them this turn and is sacrificed at the end step." },
  { name: 'Ninjutsu', text: 'Pay the cost and return an unblocked attacker you control to hand: put this card onto the battlefield tapped and attacking.' },
  { name: 'Dash', text: "Cast it for its dash cost: it gains haste and returns to its owner's hand at the next end step." },
  { name: 'Blitz', text: 'Cast it for its blitz cost: it gains haste and "When this dies, draw a card," and is sacrificed at the next end step.' },
  { name: 'Evoke', text: 'Cast it for its evoke cost: it is sacrificed as soon as it enters (its enter abilities still happen).' },
  { name: 'Emerge', text: 'Cast it by sacrificing a creature and paying the emerge cost reduced by that creature\'s mana value.' },
  { name: 'Prototype', text: 'Can be cast as a smaller version with the prototype cost, power and toughness.' },
  { name: 'Mutate', text: 'Cast it for its mutate cost on top of or under a non-Human creature you own; they merge into one creature with all abilities.' },
  { name: 'Kicker', text: 'An optional extra cost you may pay as you cast it for an additional effect.', forms: ['kicked'] },
  { name: 'Multikicker', text: 'An optional extra cost you may pay any number of times as you cast it.' },
  { name: 'Entwine', text: 'Pay the entwine cost to choose all modes instead of one.' },
  { name: 'Buyback', text: 'Pay the buyback cost as you cast it to return it to your hand instead of the graveyard.' },
  { name: 'Flashback', text: 'You may cast it from your graveyard for its flashback cost; then it is exiled.' },
  { name: 'Jump-start', text: 'You may cast it from your graveyard by paying its cost and discarding a card; then it is exiled.' },
  { name: 'Retrace', text: 'You may cast it from your graveyard by discarding a land card in addition to its cost.' },
  { name: 'Escape', text: 'You may cast it from your graveyard for its escape cost, which includes exiling other cards from your graveyard.', strict: true },
  { name: 'Disturb', text: 'You may cast it transformed from your graveyard for its disturb cost.' },
  { name: 'Aftermath', text: 'The second half can be cast only from your graveyard; then it is exiled.' },
  { name: 'Rebound', text: 'If cast from your hand, exile it as it resolves; at your next upkeep you may cast it from exile for free.' },
  { name: 'Madness', text: 'If you discard it, you may cast it for its madness cost instead of putting it into your graveyard.' },
  { name: 'Miracle', text: 'If it is the first card you drew this turn, you may reveal it and cast it for its miracle cost.' },
  { name: 'Foretell', text: 'During your turn, pay {2} to exile it face down; cast it on a later turn for its foretell cost.' },
  { name: 'Plot', text: 'Pay the plot cost to exile it; on a later turn you may cast it for free as a sorcery.' },
  // Duskmourn
  { name: 'Impending', text: "You may cast it for its impending cost instead. Then it enters with that many time counters and isn't a creature while it has any. At the beginning of your end step, remove one of them." },
  { name: 'Suspend', text: 'Pay the suspend cost to exile it with time counters. At your upkeep remove one; when the last is removed, cast it for free.' },
  { name: 'Cycling', text: 'Pay the cycling cost and discard it: draw a card.', forms: ['cycle', 'cycles', 'cycled', 'landcycling', 'basic landcycling', 'plainscycling', 'islandcycling', 'swampcycling', 'mountaincycling', 'forestcycling'] },
  { name: 'Channel', text: 'Pay the cost and discard it from your hand for the stated effect.', strict: true },
  { name: 'Convoke', text: 'Each creature you tap while casting it pays for {1} or one mana of that creature\'s color.' },
  { name: 'Delve', text: 'Each card you exile from your graveyard while casting it pays for {1}.' },
  { name: 'Improvise', text: 'Each artifact you tap while casting it pays for {1}.' },
  { name: 'Affinity', text: 'Costs {1} less for each permanent you control of the stated kind.' },
  { name: 'Cascade', text: 'When you cast it, exile cards from the top of your library until you hit a cheaper nonland card; you may cast it for free.' },
  { name: 'Storm', text: 'When you cast it, copy it for each spell cast before it this turn.', strict: true },
  { name: 'Replicate', text: 'Pay the replicate cost any number of times as you cast it; copy it that many times.' },
  { name: 'Splice', text: 'As you cast a spell of the stated type, you may reveal this and pay its splice cost to add its text to that spell.', strict: true },
  { name: 'Overload', text: 'Cast it for its overload cost to change "target" to "each".' },
  { name: 'Spectacle', text: 'You may cast it for its spectacle cost if an opponent lost life this turn.' },
  { name: 'Surge', text: 'You may cast it for its surge cost if you or a teammate cast another spell this turn.' },
  { name: 'Casualty', text: 'As you cast it, you may sacrifice a creature with the stated power or greater to copy it.' },
  { name: 'Bargain', text: 'As you cast it, you may sacrifice an artifact, enchantment or token for an extra effect.', forms: ['bargained'] },
  { name: 'Offspring', text: 'Pay the extra offspring cost as you cast it to also create a 1/1 token copy of it when it enters.' },
  { name: 'Squad', text: 'Pay the squad cost any number of times as you cast it; create that many token copies when it enters.' },
  { name: 'Sunburst', text: 'Enters with a counter for each color of mana spent to cast it.' },
  { name: 'Fabricate', text: 'When it enters, put that many +1/+1 counters on it or create that many 1/1 Servo artifact creature tokens.' },
  { name: 'Exploit', text: 'When it enters, you may sacrifice a creature.' },
  { name: 'Extort', text: 'Whenever you cast a spell, you may pay {W/B}; if you do, each opponent loses 1 life and you gain that much.' },
  { name: 'Champion', text: 'When it enters, sacrifice it unless you exile another creature of the stated kind; that card returns when this leaves.', strict: true },
  { name: 'Hideaway', text: 'When it enters, look at the top cards of your library, exile one face down, and put the rest on the bottom.' },
  { name: 'Tribute', text: 'As it enters, an opponent chooses: put the +1/+1 counters on it, or let its "if tribute wasn\'t paid" ability happen.' },
  { name: 'Equip', text: 'Pay the cost: attach this Equipment to target creature you control. Only as a sorcery.' },
  { name: 'Reconfigure', text: 'Pay the cost: attach to target creature you control or unattach. While attached it isn\'t a creature. Only as a sorcery.' },
  { name: 'Crew', text: 'Tap any number of creatures you control with that much total power: this Vehicle becomes an artifact creature until end of turn.', forms: ['crews', 'crewed'] },
  { name: 'Saddle', text: 'Tap creatures with that much total power: it becomes saddled until end of turn. Only as a sorcery.' },
  { name: 'Enchant', text: 'An Aura can be attached only to the stated kind of object.' },
  { name: 'Fortify', text: 'Pay the cost: attach this Fortification to target land you control. Only as a sorcery.' },
  { name: 'Daybound', text: 'If a player casts no spells during their own turn, it becomes night next turn and this transforms.' },
  { name: 'Nightbound', text: 'If a player casts two or more spells during a turn, it becomes day next turn and this transforms.' },
  { name: 'Mobilize', text: 'Whenever it attacks, create that many tapped and attacking 1/1 red Warrior tokens; sacrifice them at the next end step.' },
];

/** Game actions and things cards create; matched anywhere in the text. */
const ACTIONS: Entry[] = [
  { name: 'Scry', text: 'Look at that many cards from the top of your library; put any of them on the bottom and the rest back on top in any order.', forms: ['scries'] },
  { name: 'Surveil', text: 'Look at that many cards from the top of your library; put any of them into your graveyard and the rest back on top in any order.', forms: ['surveils'] },
  { name: 'Mill', text: 'Put that many cards from the top of the library into the graveyard.', forms: ['mills', 'milled'] },
  { name: 'Fight', text: 'Each of the two creatures deals damage equal to its power to the other.', forms: ['fights'] },
  { name: 'Investigate', text: 'Create a Clue token: an artifact with "{2}, Sacrifice this: Draw a card."', forms: ['investigates'] },
  { name: 'Explore', text: 'Reveal the top card of your library. If it is a land, put it in your hand; otherwise put a +1/+1 counter on the creature and you may put the card into your graveyard.', forms: ['explores'] },
  { name: 'Proliferate', text: 'Choose any number of permanents and players with counters; give each another counter of a kind it already has.' },
  { name: 'Connive', text: 'Draw a card, then discard a card. If you discarded a nonland card, put a +1/+1 counter on the creature.', forms: ['connives'] },
  { name: 'Amass', text: 'Put that many +1/+1 counters on an Army you control; create a 0/0 Army token first if you have none.', forms: ['amasses'] },
  { name: 'Adapt', text: 'If it has no +1/+1 counters, put that many +1/+1 counters on it.', forms: ['adapts'] },
  { name: 'Monstrosity', text: 'If it isn\'t monstrous, put that many +1/+1 counters on it and it becomes monstrous.', forms: ['monstrous'] },
  { name: 'Transform', text: 'Turn a double-faced card over to its other face.', forms: ['transforms', 'transformed'] },
  { name: 'Discover', text: 'Exile cards from the top of your library until you hit a nonland card with that mana value or less; cast it for free or put it in your hand.', forms: ['discovers'] },
  // before Manifest, which must not claim "manifest dread"
  { name: 'Manifest dread', text: 'Look at the top two cards of your library. Put one onto the battlefield face down as a 2/2 creature and the other into your graveyard. If the face-down card is a creature card, you may turn it face up any time for its mana cost.', forms: ['manifests dread'] },
  { name: 'Manifest', text: 'Put the card onto the battlefield face down as a 2/2 creature; if it is a creature card you may turn it face up for its mana cost.', match: /\bmanifests?\b(?!\s+dread)/i },
  // Duskmourn
  { name: 'Face-down', text: "A face-down permanent is a 2/2 creature with no name, text, color or mana cost. Its controller may look at it; other players can't. A face-down creature card can be turned face up for its mana cost.", match: /\bface[- ](down|up)\b/i },
  { name: 'Room', text: 'An enchantment with two doors, each with its own name, cost and abilities. Cast either door: it enters unlocked. Later, as a sorcery, you may pay a locked door\'s cost to unlock it. Unlocking the second door fully unlocks the Room, which triggers eerie.', match: /\b(rooms?|unlock(s|ed)?|(locked )?doors?)\b/i },
  { name: 'Stun counter', text: 'If a permanent with a stun counter would untap, remove a stun counter from it instead.', match: /\bstun counters?\b/i, counter: 'stun' },
  { name: 'Finality counter', text: 'If a permanent with a finality counter would go to a graveyard from the battlefield, exile it instead.', match: /\bfinality counters?\b/i, counter: 'finality' },
  { name: 'Populate', text: 'Create a copy of a creature token you control.', forms: ['populates'] },
  { name: 'Goad', text: 'Until your next turn, the goaded creature attacks each combat if able, and attacks a player other than you if able.', forms: ['goads', 'goaded'] },
  { name: 'Venture into the dungeon', text: 'Move to the next room of a dungeon (or start one) and get that room\'s effect.', forms: ['ventures into the dungeon'] },
  { name: 'Treasure', text: 'An artifact token with "{T}, Sacrifice this: Add one mana of any color."' },
  { name: 'Food', text: 'An artifact token with "{2}, {T}, Sacrifice this: You gain 3 life."' },
  { name: 'Clue', text: 'An artifact token with "{2}, Sacrifice this: Draw a card."' },
  { name: 'Blood', text: 'An artifact token with "{1}, {T}, Discard a card, Sacrifice this: Draw a card."', match: /\bblood tokens?\b/i },
  { name: 'Map', text: 'An artifact token with "{1}, {T}, Sacrifice this: Target creature you control explores. Only as a sorcery."', match: /\bmap tokens?\b/i },
  { name: 'Sacrifice', text: 'Put a permanent you control into its owner\'s graveyard. It isn\'t destroyed, so indestructible doesn\'t stop it.', forms: ['sacrifices', 'sacrificed'] },
  { name: 'Counter', text: 'A countered spell or ability does nothing; a countered spell goes to its owner\'s graveyard.', match: /\bcounter (target|that|it\b|all|each)[^.]*?\b(spell|ability)/i },
  { name: 'Exile', text: 'Remove from the game into the exile zone. Cards there usually stay there.', forms: ['exiles', 'exiled'] },
];

const ROOM = ACTIONS.find((entry) => entry.name === 'Room')!;
const FACE_DOWN = ACTIONS.find((entry) => entry.name === 'Face-down')!;

/** Ability words name a group of abilities but have no rules meaning of their own; the text after the dash says it. */
const ABILITY_WORDS: Entry[] = [
  { name: 'Landfall', text: 'Happens whenever a land enters the battlefield under your control.' },
  { name: 'Raid', text: 'Checks whether you attacked this turn.' },
  { name: 'Morbid', text: 'Checks whether a creature died this turn.' },
  { name: 'Revolt', text: 'Checks whether a permanent you controlled left the battlefield this turn.' },
  { name: 'Ferocious', text: 'Checks whether you control a creature with power 4 or greater.' },
  { name: 'Metalcraft', text: 'Checks whether you control three or more artifacts.' },
  { name: 'Threshold', text: 'Checks whether seven or more cards are in your graveyard.' },
  { name: 'Delirium', text: 'Checks whether there are four or more card types among cards in your graveyard.' },
  { name: 'Constellation', text: 'Happens whenever an enchantment enters under your control.' },
  { name: 'Magecraft', text: 'Happens whenever you cast or copy an instant or sorcery spell.' },
  { name: 'Heroic', text: 'Happens whenever you cast a spell that targets it.' },
  { name: 'Spell mastery', text: 'Checks whether there are two or more instant and/or sorcery cards in your graveyard.' },
  { name: 'Hellbent', text: 'Checks whether you have no cards in hand.' },
  { name: 'Coven', text: 'Checks whether you control three or more creatures with different powers.' },
  { name: 'Alliance', text: 'Happens whenever another creature enters under your control.' },
  { name: 'Celebration', text: 'Checks whether two or more nonland permanents entered under your control this turn.' },
  { name: 'Domain', text: 'Counts the basic land types among lands you control.' },
  { name: 'Formidable', text: 'Checks whether creatures you control have total power 8 or greater.' },
  { name: 'Battalion', text: 'Happens whenever it and at least two other creatures attack.' },
  { name: 'Inspired', text: 'Happens whenever it becomes untapped.' },
  { name: 'Eerie', text: 'Happens whenever an enchantment enters under your control or you fully unlock a Room.' },
  { name: 'Survival', text: 'Happens at the beginning of your second main phase if it is tapped.' },
  { name: "Council's dilemma", text: 'Each player votes, starting with you; each vote adds to the effect of what it voted for.' },
];

export type LineKind = 'keyword' | 'triggered' | 'activated' | 'loyalty' | 'chapter' | 'spell' | 'static';

export interface RulesLine {
  /** the line as printed, markup and mana symbols intact */
  text: string;
  kind: LineKind;
  /** an ability word that leads the line (Landfall, Raid...) */
  abilityWord?: string;
}

/** What the server says is true of the card right now ("Revolt: a permanent left the battlefield this turn"). */
export interface Hint {
  text: string;
  tone: 'good' | 'bad' | 'neutral';
}

export interface CardExplanation {
  lines: RulesLine[];
  terms: Term[];
  hints: Hint[];
}

/** XMage appends live hints to the rules after this mark; each starts with an icon name. */
const HINT_MARK = '<hintstart/>';
const HINT_ICON = /^ICON_(DUNGEON_ROOM_CURRENT|DUNGEON_ROOM_NEXT|RESTRICT|REQUIRE|GOOD|BAD)/;

const DASH = /\s*[—–]\s*/;
const COSTS = /^(\{[^}]+\}|tap|untap|sacrifice|discard|pay|exile|remove|return|reveal|put|collect|forage|waterbend|[+−-]?\d+|x)/i;

function escape(value: string) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

function plain(text: string): string {
  // reminder text in parentheses explains itself; it mustn't make a keyword look present
  return stripMarkup(text).replace(/\([^)]*\)/g, ' ').replace(/\s+/g, ' ').trim();
}

/** Words that make a sentence: after a keyword they mean the line merely starts with the same word (a card name). */
const VERBS = /\b(deals?|gets?|gains?|has|have|is|are|can|can't|enters?|dies|attacks?|blocks?|becomes?|draws?|loses?|returns?|creates?|destroys?)\b/i;

/** The keyword at the start of each comma- or semicolon-separated part of a keyword line. */
function leadingKeywords(line: string): Entry[] {
  const parts = line.split(/[,;]/).map((part) => part.trim().toLowerCase());
  return KEYWORDS.filter((entry) => parts.some((part) => [entry.name.toLowerCase(), ...(entry.forms ?? [])].some((form) => {
    if (part === form) return true;
    if (!part.startsWith(form)) return false;
    const rest = part.slice(form.length);
    // a cost or parameter may follow ("Ward {2}", "Protection from red", "Annihilator 2"), not a sentence
    return /^(\s|\{|—|-)/.test(rest) && !VERBS.test(rest) && rest.trim().split(/\s+/).length <= 6;
  })));
}

function mentions(text: string, entry: Entry): boolean {
  if (entry.match) return entry.match.test(text);
  return [entry.name, ...(entry.forms ?? [])].some((form) => new RegExp(`\\b${escape(form)}\\b`, 'i').test(text));
}

function isKeywordLine(line: string): boolean {
  const parts = line.split(/[,;]/).map((part) => part.trim()).filter(Boolean);
  return parts.length > 0 && parts.every((part) => leadingKeywords(part).length > 0);
}

function classify(line: string, card: CardView): Omit<RulesLine, 'text'> {
  const text = plain(line);
  let body = text;
  let abilityWord: string | undefined;
  const dashed = text.split(DASH);
  if (dashed.length > 1 && dashed[0].split(' ').length <= 3 && !/[{:]/.test(dashed[0])) {
    const word = ABILITY_WORDS.find((entry) => entry.name.toLowerCase() === dashed[0].toLowerCase());
    if (word) {
      abilityWord = word.name;
      body = dashed.slice(1).join(' — ');
    }
  }
  const types = card.cardTypes ?? [];
  if (/^(chapter|[ivx]+(, [ivx]+)*\s*[—–])/i.test(body)) return { kind: 'chapter', abilityWord };
  if (/^[+−-]?\d+:/.test(body) || /^[+−-]x:/i.test(body)) return { kind: 'loyalty', abilityWord };
  if (/^(when|whenever|at the beginning|at the end)\b/i.test(body)) return { kind: 'triggered', abilityWord };
  const colon = body.indexOf(':');
  if (colon > 0 && colon < 90 && COSTS.test(body)) return { kind: 'activated', abilityWord };
  if (isKeywordLine(body)) return { kind: 'keyword', abilityWord };
  if (types.includes('INSTANT') || types.includes('SORCERY')) return { kind: 'spell', abilityWord };
  return { kind: 'static', abilityWord };
}

/** The card's own name in place of "{this}". */
function named(text: string, card: CardView): string {
  return text.replace(/\{this\}/gi, card.displayName ?? card.name ?? 'This');
}

/** The rules lines of a card, and the live hints the server appended after them. */
export function splitRules(card: CardView): { rules: string[]; hints: Hint[] } {
  const rules: string[] = [];
  const hintLines: string[] = [];
  let inHints = false;
  for (const raw of card.rules ?? []) {
    const line = named(raw, card);
    const mark = line.indexOf(HINT_MARK);
    if (!inHints && mark >= 0) {
      inHints = true;
      rules.push(line.slice(0, mark));
      hintLines.push(line.slice(mark + HINT_MARK.length));
    } else {
      (inHints ? hintLines : rules).push(line);
    }
  }
  const hints = hintLines.flatMap((line) => line.split(/<br\s*\/?>/i)).map((line) => {
    const text = stripMarkup(line);
    const icon = HINT_ICON.exec(text)?.[1];
    return {
      text: icon ? text.slice(`ICON_${icon}`.length).trim() : text,
      tone: icon === 'GOOD' ? 'good' as const : icon === 'BAD' || icon === 'RESTRICT' ? 'bad' as const : 'neutral' as const,
    };
  }).filter((hint) => hint.text.length > 0);
  return { rules: rules.filter((line) => plain(line).length > 0), hints };
}

/** Rules lines of a card, without the empty ones and the server's hints. */
export function rulesOf(card: CardView): string[] {
  return splitRules(card).rules;
}

/** The names the card goes by in its own text: its name, and each half of a split card or Room */
function ownNames(card: Pick<CardView, 'name' | 'displayName'>): string[] {
  const names = new Set<string>();
  for (const name of [card.displayName, card.name]) {
    if (!name) continue;
    names.add(name);
    for (const half of name.split(/\s*\/\/\s*/)) if (half) names.add(half);
  }
  return [...names].sort((a, b) => b.length - a.length);
}

/** Each line's kind, and every keyword, action and ability word the card mentions (in order of appearance). */
export function explainCard(card: CardView): CardExplanation {
  const { rules, hints } = splitRules(card);
  const lines = rules.map((text) => ({ text, ...classify(text, card) }));
  const terms: Term[] = [];
  const seen = new Set<string>();
  const add = (entry: Entry) => {
    if (seen.has(entry.name)) return;
    seen.add(entry.name);
    terms.push({ name: entry.name, text: entry.text });
  };
  // a Room's own rules are reminder text, which doesn't count: its type says it
  if ((card.subTypes ?? []).some((type) => String(type).toUpperCase() === 'ROOM')) add(ROOM);
  // a face-down permanent has no text of its own to say what it is
  if (card.faceDown) add(FACE_DOWN);
  const names = ownNames(card);
  for (const line of lines) {
    // the card's own name says nothing ("Betrayer's Bargain deals 5 damage" isn't bargain)
    const text = names.reduce((rest, name) => rest.split(name).join('This'), plain(line.text));
    if (line.abilityWord) add(ABILITY_WORDS.find((entry) => entry.name === line.abilityWord)!);
    for (const part of text.split(DASH)) leadingKeywords(part).forEach(add);
    KEYWORDS.filter((entry) => !entry.strict && mentions(text, entry)).forEach(add);
    ACTIONS.filter((entry) => mentions(text, entry)).forEach(add);
    // an ability word after a sentence of the same line ("Draw a card. Delirium — ...")
    ABILITY_WORDS.filter((entry) => new RegExp(`\\b${escape(entry.name)}\\s*[—–]`, 'i').test(text)).forEach(add);
  }
  // counters with a rule of their own, on a permanent whose text doesn't mention them (a stunned creature)
  for (const counter of (card as { counters?: { name?: string; count?: number }[] }).counters ?? []) {
    const entry = ACTIONS.find((candidate) => candidate.counter && candidate.counter === counter.name?.toLowerCase());
    if (entry && (counter.count ?? 0) > 0) add(entry);
  }
  return { lines, terms, hints };
}

/** A piece of rules text, and the keyword, action or ability word it names (if it names one). */
export interface TermSpan {
  text: string;
  term?: Term;
}

function formsPattern(entry: Entry): RegExp {
  if (entry.match) return new RegExp(entry.match.source, entry.match.flags.includes('g') ? entry.match.flags : `${entry.match.flags}g`);
  const forms = [entry.name, ...(entry.forms ?? [])].sort((a, b) => b.length - a.length).map(escape);
  return new RegExp(`\\b(${forms.join('|')})\\b`, 'gi');
}

/**
 * Rules text cut into the stretches that name a keyword, action or ability word, and the plain text between,
 * so each named term can be explained where it is printed. The same rules as explainCard: reminder text (in
 * parentheses) names nothing, a strict keyword counts only where it leads a keyword list, and an ability word only
 * before its dash; given the card, its own name names nothing either. Joined back together the spans are the text
 * unchanged.
 */
export function termSpans(text: string, card?: Pick<CardView, 'name' | 'displayName'>): TermSpan[] {
  // reminder text, and the card's own name where it is printed (passing the card), name nothing
  const reminders: [number, number][] = [];
  for (const found of text.matchAll(/\([^)]*\)/g)) reminders.push([found.index, found.index + found[0].length]);
  for (const name of card ? ownNames(card) : []) {
    for (let at = text.indexOf(name); at >= 0; at = text.indexOf(name, at + name.length)) reminders.push([at, at + name.length]);
  }
  const inReminder = (start: number) => reminders.some(([from, to]) => start >= from && start < to);
  const hits: { start: number; end: number; entry: Entry }[] = [];
  const scan = (entry: Entry, accept: (start: number, end: number) => boolean) => {
    for (const found of text.matchAll(formsPattern(entry))) {
      const start = found.index;
      const end = start + found[0].length;
      if (found[0].length > 0 && !inReminder(start) && accept(start, end)) hits.push({ start, end, entry });
    }
  };
  for (const entry of KEYWORDS) {
    scan(entry, (start) => {
      if (!entry.strict) return true;
      // the comma- or dash-separated part it starts must read as a keyword ("Shadow", not "Shadow of Doubt deals")
      const before = text.slice(0, start);
      if (!/(^|[,;—–.]\s*)$/.test(before)) return false;
      const part = text.slice(start).split(/[,;—–(.]/)[0];
      return leadingKeywords(part).includes(entry);
    });
  }
  for (const entry of ACTIONS) scan(entry, () => true);
  for (const entry of ABILITY_WORDS) scan(entry, (_, end) => /^\s*[—–-]/.test(text.slice(end)));
  // the earliest wins, then the longest ("Manifest dread" over "Manifest")
  hits.sort((a, b) => a.start - b.start || b.end - a.end);
  const spans: TermSpan[] = [];
  let at = 0;
  for (const hit of hits) {
    if (hit.start < at) continue;
    if (hit.start > at) spans.push({ text: text.slice(at, hit.start) });
    spans.push({ text: text.slice(hit.start, hit.end), term: { name: hit.entry.name, text: hit.entry.text } });
    at = hit.end;
  }
  if (at < text.length) spans.push({ text: text.slice(at) });
  return spans;
}

/** Keywords the card has as abilities of its own (from its keyword lines), not ones it merely mentions. */
export function keywordsOf(card: CardView): string[] {
  const names = new Set<string>();
  for (const line of splitRules(card).rules) {
    const text = plain(line);
    if (classify(line, card).kind !== 'keyword') continue;
    for (const part of text.split(DASH)) leadingKeywords(part).forEach((entry) => names.add(entry.name));
  }
  return [...names];
}

export const LINE_KIND_LABEL: Record<LineKind, string> = {
  keyword: 'Keyword',
  triggered: 'Triggered',
  activated: 'Activated',
  loyalty: 'Loyalty',
  chapter: 'Chapter',
  spell: 'Effect',
  static: 'Static',
};

export const LINE_KIND_HELP: Record<LineKind, string> = {
  keyword: 'Keywords are shorthand for common rules; see Mechanics.',
  triggered: 'Triggered: happens by itself when its event occurs, using the stack, so players can respond.',
  activated: 'Activated: you choose to use it by paying the cost before the colon.',
  loyalty: 'Loyalty: once per turn, as a sorcery, add or remove that many loyalty counters to use it.',
  chapter: 'Chapter: a lore counter is added each of your precombat main phases; each chapter triggers when it is reached.',
  spell: 'What the spell does when it resolves.',
  static: 'Static: always true while the card is in play.',
};
