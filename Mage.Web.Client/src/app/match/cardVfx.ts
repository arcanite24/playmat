import type { CardView, GameEventView } from '../../protocol/generated/views';

/**
 * Signature effects (#226): the spells and abilities people play most get an effect of their own when they resolve, the
 * way Arena does it. A Lightning Bolt cracks a fork of lightning from its caster to what it hits, Counterspell slams
 * a glyph onto the spell it counters, Wrath of God sweeps the battlefield in white light.
 *
 * Effects are built from a few primitives (SignatureVfx.tsx draws them), and a card gets one by name in SIGNATURES
 * below: adding a card is one line. The list started from the cards most played on playmat.deimos.studio
 * (300 games and 358 saved decks read on 2026-11-07: the mill, zombie, goblin, aura, elf and Commander decks, plus the
 * removal, counters and cantrips every deck plays) and the format staples the AI decks play. An instant or sorcery
 * without an entry still gets a plainer effect in its color, read from what it did (see signatureFor).
 */

export type Primitive =
  /** a forked arc from the caster to each target, and a white flash where it lands */
  | 'lightning'
  /** a ball of fire thrown at each target, bursting where it lands */
  | 'fire'
  /** a pillar of light coming down on each target */
  | 'beam'
  /** a glyph ring slammed on the spell it counters, which shatters */
  | 'counter'
  /** a wave of ink or light across the whole battlefield */
  | 'sweep'
  /** tendrils closing on each target */
  | 'tendrils'
  /** a pulse of leaves and growth rings on each target (or the caster) */
  | 'growth'
  /** a halo settling over the enchanted creature */
  | 'halo'
  /** a whirl lifting each target away */
  | 'bounce'
  /** runes circling the caster: cards drawn, looked at, searched for */
  | 'insight'
  /** card backs spilling from the target player (or the caster) into their graveyard */
  | 'mill'
  /** a wisp pulled out of the target player's hand */
  | 'mind'
  /** motes of mana spiralling into the caster */
  | 'ritual'
  /** rays and rising motes where creatures are called onto the battlefield */
  | 'summon'
  /** a stream of life pulled from the targets (or opponents) to the caster */
  | 'drain'
  /** claws raking each target: fight and bite spells */
  | 'claws';

/** The mat's inks the effects are printed in (defined in the theme), or a color of their own. */
export type Ink = 'red' | 'blue' | 'black' | 'white' | 'green' | 'gold' | 'void' | 'mana';

export interface Signature {
  fx: Primitive;
  ink: Ink;
  /** bigger, longer, and the table shakes */
  big?: boolean;
}

const sig = (fx: Primitive, ink: Ink, big = false): Signature => (big ? { fx, ink, big } : { fx, ink });

const each = (names: string[], signature: Signature): [string, Signature][] => names.map((name) => [name, signature]);

/** Card (or ability source) name to its effect. */
export const SIGNATURES: ReadonlyMap<string, Signature> = new Map<string, Signature>([
  // burn
  ['Lightning Bolt', sig('lightning', 'red', true)],
  ['Lightning Helix', sig('lightning', 'gold', true)],
  ['Chain Lightning', sig('lightning', 'red', true)],
  ...each(['Burst Lightning', 'Shock', 'Lightning Strike', 'Galvanic Discharge', 'Wild Slash', 'Play with Fire', 'Spikefield Hazard', 'Galvanic Blast', 'Electrickery', 'Hypothesizzle', 'Lightning Axe'], sig('lightning', 'red')),
  ...each(['Fireball', 'Banefire', 'Stoke the Flames', 'Skewer the Critics', 'Fiery Confluence', 'Bonecrusher Giant', 'Stomp', 'Lava Dart'], sig('fire', 'red', true)),
  ...each(['Searing Spear', 'Searing Blaze', 'Lava Spike', 'Rift Bolt', 'Shrapnel Blast', 'Abrade', 'Fiery Impulse', 'Flame Slash', 'Incinerate', 'Lightning Blast', 'Volcanic Hammer', 'Collective Defiance', 'Boros Charm', "Kolaghan's Command", 'Fire // Ice', 'Fire', 'Fling'], sig('fire', 'red')),
  // counters
  ['Counterspell', sig('counter', 'blue', true)],
  ['Mana Drain', sig('counter', 'blue', true)],
  ['Force of Will', sig('counter', 'blue', true)],
  ...each(['Mana Leak', 'Dissolve', 'Dissipate', 'Arcane Denial', 'Negate', 'Swan Song', 'Delay', 'Essence Scatter', 'Remand', 'Force of Negation', 'Spell Pierce', 'Make Disappear', 'Absorb', 'Memory Lapse', 'Lose Focus', 'Saw It Coming', 'Stern Scolding', 'Jwari Disruption', 'Daze', 'Mystical Dispute', 'Syncopate', 'No More Lies', 'Cancel', 'Sublime Epiphany', 'An Offer You Can\'t Refuse', 'Override', 'Drown in the Loch', 'Fierce Guardianship', 'Pact of Negation', 'Flusterstorm', 'Miscalculation', 'Disdainful Stroke', 'Didn\'t Say Please', 'Change the Equation', 'Three Steps Ahead', 'Spell Snare', 'Logic Knot'], sig('counter', 'blue')),
  // exile and other removal from light
  ['Swords to Plowshares', sig('beam', 'white', true)],
  ['Path to Exile', sig('beam', 'white', true)],
  ...each(['Unexpectedly Absent', 'Leyline Binding', 'Fateful Absence', 'Generous Gift', 'Prismatic Ending', 'March of Otherworldly Light', 'Skyclave Apparition', 'Forsake the Worldly', 'Oblivion Ring', 'Banishing Light', 'Portable Hole', 'Get Lost', 'Solitude', 'Ossification', 'Council\'s Judgment', 'Valorous Stance', 'Journey to Nowhere', 'Ephemerate', 'Lay Down Arms'], sig('beam', 'white')),
  ...each(['Vindicate', 'Anguished Unmaking', 'Despark', 'Assassin\'s Trophy', 'Kaya\'s Guile', 'Tear Asunder'], sig('beam', 'gold')),
  ...each(['Pongify', 'Rapid Hybridization', 'Beast Within', 'Chaos Warp', 'Reality Shift'], sig('beam', 'green')),
  // removal from ink
  ...each(['Doom Blade', 'Go for the Throat', 'Murder', 'Hero\'s Downfall', 'Infernal Grasp', 'Bitter Triumph', 'Cut Down', 'Power Word Kill', 'Fatal Push', 'Bloodchief\'s Thirst', 'Heartless Act', 'Lethal Scheme', 'Dismember', 'Snuff Out', 'Terminate', 'Dreadbore', 'Eat to Extinction', 'Sheoldred\'s Edict', 'Feed the Swarm', 'Tragic Slip', 'Victim of Night', 'Dark Salvation', 'Cast Down', 'Baleful Ammit'], sig('tendrils', 'black')),
  ...each(['Void Rend', 'Spatial Contortion', 'Scour from Existence', 'Not of This World'], sig('tendrils', 'void')),
  // fight and bite
  ...each(['Prey Upon', 'Bite Down', 'Rabid Bite', 'Ram Through', 'Infuriate', 'Tail Swipe', 'Primal Might', 'Pounce', 'Fight with Fire', 'Ancient Animus', 'Savage Swipe'], sig('claws', 'green')),
  // sweepers
  ['Wrath of God', sig('sweep', 'white', true)],
  ['Day of Judgment', sig('sweep', 'white', true)],
  ['Supreme Verdict', sig('sweep', 'white', true)],
  ['Damnation', sig('sweep', 'black', true)],
  ['Toxic Deluge', sig('sweep', 'black', true)],
  ['Blasphemous Act', sig('sweep', 'red', true)],
  ['Farewell', sig('sweep', 'white', true)],
  ['Sunfall', sig('sweep', 'white', true)],
  ['Cyclonic Rift', sig('sweep', 'blue', true)],
  ...each(['Cleansing Nova', 'Final Judgment', 'Depopulate', 'Austere Command', 'Doomskar', 'Temporary Lockdown', 'Fumigate', 'Hallowed Burial', 'Planar Cleansing', 'Teferi\'s Protection', 'Heroic Intervention', 'Inspiring Roar', 'Settle the Wreckage'], sig('sweep', 'white')),
  ...each(['Crux of Fate', 'Damn', 'Languish', 'Extinction Event', 'Deadly Tempest', 'Massacre Wurm', 'Pestilence', 'Shadows\' Verdict', 'Culling Ritual'], sig('sweep', 'black')),
  ...each(['Anger of the Gods', 'Pyroclasm', 'Sweltering Suns', 'Kozilek\'s Return', 'Vandalblast', 'Storm\'s Wrath', 'Brotherhood\'s End', 'Hour of Devastation', 'Slagstorm', 'Fiery Cannonade', 'Shatter the Sky'], sig('sweep', 'red')),
  ...each(['Evacuation', 'Aetherize', 'Engulf the Shore', 'Whelming Wave'], sig('sweep', 'blue')),
  ...each(['Craterhoof Behemoth', 'Overrun', 'Triumph of the Hordes', 'Finale of Devastation'], sig('sweep', 'green', true)),
  // bounce
  ...each(['Capsize', 'Echoing Truth', 'Blink of an Eye', 'Unsummon', 'Into the Roil', 'Repulse', 'Petty Theft', 'Fading Hope', 'Bounce Off', 'Snap', 'Boomerang', 'Vapor Snag', 'Brazen Borrower', 'Aether Gust', 'Sink into Stupor', 'Consign // Oblivion', 'Commit // Memory', 'Run Away Together'], sig('bounce', 'blue')),
  // cards drawn, looked at, searched for
  ...each(['Fact or Fiction', 'Dig Through Time', 'Treasure Cruise', 'Stroke of Genius', 'Mystic Remora', 'Rhystic Study', 'Jeska\'s Will', 'Expressive Iteration', 'Pull from Tomorrow', 'Blue Sun\'s Zenith', 'Mind\'s Desire'], sig('insight', 'blue', true)),
  ...each(['Brainstorm', 'Ponder', 'Preordain', 'Sleight of Hand', 'Serum Visions', 'Consider', 'Opt', 'Impulse', 'Accumulate Wisdom', 'Deep Analysis', 'Into the Story', 'Visions of Beyond', 'Stock Up', 'Consult the Star Charts', 'Frantic Search', 'Esper Origins', 'Open into Wonder', 'Waterbending Lesson', 'Solstice Revelations', 'Mystical Tutor', 'Faithless Looting', 'Think Twice', 'Glimpse the Unseen', 'Divination', 'Chart a Course', 'Behold the Multitude', 'Lórien Revealed', 'Fblthp, the Lost', 'Thirst for Discovery', 'Quick Study', 'Artful Dodge'], sig('insight', 'blue')),
  ...each(['Harmonize', 'Ancient Stirrings', 'Sylvan Library', 'Once Upon a Time', 'Commune with Nature', 'Lead the Stampede', 'Worldly Tutor', 'Green Sun\'s Zenith'], sig('insight', 'green')),
  ...each(['Demonic Tutor', 'Vampiric Tutor', 'Diabolic Intent', 'Night\'s Whisper', 'Sign in Blood', 'Read the Bones', 'Painful Truths', 'Village Rites', 'Deadly Dispute', 'Bitter Reunion', 'Imperial Seal', 'Grim Tutor'], sig('insight', 'black')),
  ...each(['Enlightened Tutor', 'Idyllic Tutor', 'Recruitment Officer'], sig('insight', 'white')),
  // mill
  ['Glimpse the Unthinkable', sig('mill', 'blue', true)],
  ['Mind Funeral', sig('mill', 'blue', true)],
  ['Maddening Cacophony', sig('mill', 'blue', true)],
  ['Archive Trap', sig('mill', 'blue', true)],
  ...each(['Hedron Crab', 'Fraying Sanity', 'Breaking', 'Breaking // Entering', 'Ruin Crab', 'Merfolk Secretkeeper', 'Thought Scour', 'Mind Grind', 'Tasha\'s Hindrance', 'Visions of Dread', 'Tome Scour', 'Startled Awake', 'Jace\'s Erasure', 'Sphinx\'s Tutelage', 'Psychic Corrosion', 'Bruvac the Grandiloquent', 'Mesmeric Orb', 'Teferi\'s Tutelage', 'Persistent Petitioners', 'Crabomination', 'Bloodchief Ascension'], sig('mill', 'blue')),
  // discard
  ...each(['Thoughtseize', 'Duress', 'Inquisition of Kozilek', 'Hymn to Tourach', 'Mind Rot', 'Kitesail Freebooter', 'Grief', 'Gitaxian Probe', 'Unmask', 'Cabal Therapy', 'Thought Distortion', 'Mind Twist', 'Syphon Mind', 'Liliana of the Veil'], sig('mind', 'void')),
  // mana
  ...each(['Dark Ritual', 'Cabal Ritual', 'Seething Song', 'Pyretic Ritual', 'Desperate Ritual', 'Rite of Flame', 'Manamorphose', 'Brightstone Ritual', 'Burnt Offering', 'Culling the Weak'], sig('ritual', 'mana')),
  ...each(['Rampant Growth', 'Cultivate', 'Kodama\'s Reach', 'Farseek', 'Nature\'s Lore', 'Three Visits', 'Explore', 'Harrow', 'Skyshroud Claim', 'Explosive Vegetation', 'Sakura-Tribe Elder', 'Wood Elves', 'Search for Tomorrow', 'Growth Spiral', 'Arboreal Grazer'], sig('growth', 'green')),
  // pump and protection
  ['Giant Growth', sig('growth', 'green', true)],
  ...each(['Blossoming Defense', 'Vines of Vastwood', 'Snakeskin Veil', 'Tamiyo\'s Safekeeping', 'Ranger\'s Guile', 'Mutagenic Growth', 'Engineered Might', 'Wild Ride', 'Titanic Growth', 'Brute Force', 'Become Immense', 'Groundswell', 'Might of Old Krosa', 'Unbridled Growth'], sig('growth', 'green')),
  // auras
  ...each(['Ethereal Armor', 'Spirit Mantle', 'Daybreak Coronet', 'Rancor', 'Spider Umbra', 'Ancestral Mask', 'All That Glitters', 'Sentinel\'s Eyes', 'Cartouche of Ambition', 'Cartouche of Strength', 'Cartouche of Solidarity', 'Cartouche of Knowledge', 'Cartouche of Zeal', 'Hyena Umbra', 'Utopia Sprawl', 'Rabbit Battery', 'Shield of the Oversoul', 'Eel Umbra', 'Gryff\'s Boon', 'Sigil of the Empty Throne', 'Kor Spiritdancer', 'Sram, Senior Edificer', 'Starfield Mystic', 'Light-Paws, Emperor\'s Voice', 'Sythis, Harvest\'s Hand', 'Siona, Captain of the Pyleas', 'Mesa Enchantress', 'Setessan Champion'], sig('halo', 'gold')),
  // creatures called in
  ['Collected Company', sig('summon', 'green', true)],
  ['Chord of Calling', sig('summon', 'green', true)],
  ...each(['Lingering Souls', 'Spectral Procession', 'Raise the Alarm', 'Secure the Wastes', 'Triplicate Spirits', 'Giada, Font of Hope', 'Resplendent Angel', 'Righteous Valkyrie', 'Bishop of Wings', 'Enduring Innocence', 'Momo, Friendly Flier', 'Angel of Invention', 'Lyra Dawnbringer', 'Adeline, Resplendent Cathar', 'Ajani, Valiant Protector'], sig('summon', 'white')),
  ...each(['Avenger of Zendikar', 'Elvish Promenade', 'Llanowar Elves', 'Elvish Mystic', 'Badgermole Cub', 'Ouroboroid', 'Scute Swarm', 'Tireless Provisioner', 'Gladecover Scout', 'Slippery Bogle', 'Eternal Witness', 'Trostani, Selesnya\'s Voice', 'Nissa, Genesis Mage', 'Nissa, Voice of Zendikar', 'Doubling Season', 'Parallel Lives', 'Anointed Procession'], sig('summon', 'green')),
  ...each(['Krenko, Mob Boss', 'Krenko, Tin Street Kingpin', 'Mogg War Marshal', 'Goblin Warchief', 'Goblin Matron', 'Skirk Prospector', 'Beetleback Chief', 'Siege-Gang Commander', 'Goblin Rabblemaster', 'Legion Warboss', 'Dragon Fodder', 'Hordeling Outburst', 'Goblin Instigator', 'Empty the Warrens', 'Ragavan, Nimble Pilferer', 'Chandra, Bold Pyromancer', 'Chandra, Pyrogenius', 'Impact Tremors'], sig('summon', 'red')),
  ...each(['Wight of Precinct Six', 'Cryptbreaker', 'Death Baron', 'Champion of the Perished', 'Dread Wanderer', 'Diregraf Colossus', 'Gravedigger', 'Liliana, Death Wielder', 'Lord of the Accursed', 'Undead Augur', 'Carrion Feeder', 'Gravecrawler', 'Reanimate', 'Animate Dead', 'Necromancy', 'Unearth', 'Persist', 'Living Death', 'Army of the Damned', 'Rise from the Grave', 'Zombie Apocalypse', 'Liliana\'s Influence', 'Tattered Mummy', 'Festering Mummy', 'Dune Beetle', 'Desiccated Naga', 'Decimator Beetle'], sig('summon', 'black')),
  ...each(['Talrand, Sky Summoner', 'Jace\'s Phantasm', 'Young Pyromancer', 'Murmuring Mystic', 'Tezzeret, Master of Metal'], sig('summon', 'blue')),
  // drain
  ...each(['Gray Merchant of Asphodel', 'Exsanguinate', 'Sovereign\'s Bite', 'Vito, Thorn of the Dusk Rose', 'Sanguine Bond', 'Blood Artist', 'Zulaport Cutthroat', 'Bastion of Remembrance', 'Corpse Knight', 'Cruel Celebrant', 'Marauding Blight-Priest', 'Vampire Nighthawk', 'Soul Spike', 'Kalitas, Traitor of Ghet'], sig('drain', 'black')),
  ...each(['Ajani\'s Pridemate', 'Soul Warden', 'Soul\'s Attendant', 'Healer\'s Hawk', 'Oloro, Ageless Ascetic', 'Ajani\'s Aid', 'Aid from the Cowl'], sig('drain', 'white')),
]);

const COLOR_INK: [keyof NonNullable<CardView['color']>, Ink][] = [['red', 'red'], ['blue', 'blue'], ['black', 'black'], ['white', 'white'], ['green', 'green']];

/** The card's ink: its one color, gold when it has several, white (light) when it has none. */
export function inkOf(card: Pick<CardView, 'color'>): Ink {
  const colors = COLOR_INK.filter(([key]) => card.color?.[key]).map(([, ink]) => ink);
  if (colors.length === 1) return colors[0];
  return colors.length > 1 ? 'gold' : 'white';
}

/** A name as the table keys it: a split card's half, or the whole name. */
function names(name: string): string[] {
  const halves = name.split(' // ').map((half) => half.trim()).filter(Boolean);
  return halves.length > 1 ? [name, ...halves] : [name];
}

/** What a resolved spell or ability did, as far as the effects care. */
export interface Resolution {
  /** a target was a spell or ability on the stack */
  targetedStack: boolean;
  /** it dealt damage */
  dealtDamage: boolean;
  /** permanents it targeted left the battlefield */
  removed: 'exile' | 'graveyard' | 'hand' | null;
  /** several permanents left the battlefield at once, without targets: a sweeper */
  swept: boolean;
  /** cards went from a library to a graveyard */
  milled: boolean;
  /** cards were drawn */
  drew: boolean;
}

/**
 * The effect a spell or ability gets when it resolves: its entry in SIGNATURES, else (instants and sorceries only) a
 * plain effect in its color, read from what it did. Null when it gets none (the board's usual resolve burst stays).
 */
export function signatureFor(card: CardView, did: Resolution): Signature | null {
  for (const name of names(card.name ?? '')) {
    const found = SIGNATURES.get(name);
    if (found) return found;
  }
  // abilities keep their source's name: an ability of an unlisted permanent gets no effect of its own
  if (card.mageObjectType && card.mageObjectType !== 'SPELL' && card.mageObjectType !== 'CARD') return null;
  const types = card.cardTypes ?? [];
  if (!types.includes('INSTANT') && !types.includes('SORCERY')) return null;
  const ink = inkOf(card);
  if (did.targetedStack) return sig('counter', ink === 'white' ? 'blue' : ink);
  if (did.swept) return sig('sweep', ink);
  if (did.dealtDamage) return sig('fire', ink === 'white' ? 'red' : ink);
  if (did.removed === 'hand') return sig('bounce', ink);
  if (did.removed === 'exile') return sig('beam', ink);
  if (did.removed === 'graveyard') return ink === 'black' ? sig('tendrils', 'black') : sig('beam', ink);
  if (did.milled) return sig('mill', ink);
  if (did.drew) return sig('insight', ink);
  return null;
}

/** One spell or ability that left the stack in this update, and the effect it gets. */
export interface SignatureCast {
  /** the stack object's id (where it was drawn) */
  id: string;
  name: string;
  signature: Signature;
  /** who cast or activated it */
  controllerId: string | null;
  /** what it targeted: permanents, players, spells */
  targets: string[];
  /** damage it dealt, in all */
  damage: number;
  /** players who lost cards from their library to their graveyard, and how many */
  milled: Map<string, number>;
}

/**
 * The spells and abilities that left the stack between two views, with their effects. `before` is the stack as the
 * last view had it; `events` are this view's game events (they tell what each one did; without them only listed
 * cards get an effect). A spell that was countered rather than resolved gets nothing: the counterspell's own effect
 * plays on it.
 */
export function resolvedCasts(
  before: ReadonlyMap<string, CardView>,
  nowStack: ReadonlySet<string>,
  events: readonly GameEventView[] | undefined,
  /** permanents on the battlefield before this view, to tell a sweeper (several of them leaving at once) */
  battlefieldBefore: ReadonlySet<string>,
): SignatureCast[] {
  const casts: SignatureCast[] = [];
  const departed = [...before.entries()].filter(([id]) => !nowStack.has(id));
  if (departed.length === 0) return casts;
  // a spell another one on the stack targeted and that is gone too was countered: it gets no effect of its own
  const countered = new Set<string>();
  for (const [, card] of departed) {
    for (const target of card.targets ?? []) if (before.has(target)) countered.add(target);
  }
  const list = events ?? [];
  const leftBattlefield = list.filter((event) => event.kind === 'ZONE' && event.fromZone === 'BATTLEFIELD' && event.targetId && battlefieldBefore.has(event.targetId));
  for (const [id, card] of departed) {
    if (countered.has(id)) continue;
    const targets = (card.targets ?? []).filter((target) => target !== id);
    const mine = (event: GameEventView) => event.sourceId === id || (!!card.cardId && event.sourceId === card.cardId);
    let damage = 0;
    for (const event of list) if (event.kind === 'DAMAGE' && mine(event)) damage += event.amount ?? 0;
    const milled = new Map<string, number>();
    for (const event of list) {
      if (event.kind === 'ZONE' && event.fromZone === 'LIBRARY' && event.toZone === 'GRAVEYARD' && event.playerId) {
        milled.set(event.playerId, (milled.get(event.playerId) ?? 0) + 1);
      }
    }
    const removedTargets = leftBattlefield.filter((event) => targets.includes(event.targetId!));
    const toZone = removedTargets[0]?.toZone;
    const did: Resolution = {
      targetedStack: targets.some((target) => before.has(target)),
      dealtDamage: damage > 0,
      removed: toZone === 'EXILED' ? 'exile' : toZone === 'GRAVEYARD' ? 'graveyard' : toZone === 'HAND' ? 'hand' : null,
      swept: targets.length === 0 && leftBattlefield.length >= 2,
      milled: milled.size > 0,
      drew: list.some((event) => event.kind === 'DRAW' && !!card.controllerId && event.playerId === card.controllerId),
    };
    const signature = signatureFor(card, did);
    if (!signature) continue;
    casts.push({ id, name: card.name ?? '', signature, controllerId: card.controllerId ?? null, targets, damage, milled });
  }
  return casts;
}
