import { create } from 'zustand';
import { useShallow } from 'zustand/react/shallow';
import { api } from '../connection';
import type { TurnCombatStops } from '../../core/game/autoPass';
import type { BroadcastDelay } from '../../core/game/broadcastDelay';
import { serverStops } from '../../core/game/stops';
import type { UserData, UserSkipPrioritySteps } from '../../protocol/generated/views';
import type { LocalePreference } from '../i18n/locales';
import type { CardImageRef } from '../../core/images/imageLinks';
import { readJson, writeJson } from './persist';
import { useSession } from './session';
import { DEFAULT_CARD_BACK, type CardBackId } from '../ui/cardBack';

export interface PlaySettings {
  /** tap lands automatically when the payment is unambiguous */
  autoPayMana: boolean;
  /** leave mana already in the pool for manual payment (the inverse of Use floating mana automatically) */
  autoPayRestricted: boolean;
  /** ask before passing with mana in the pool */
  confirmEmptyManaPool: boolean;
  /** order simultaneous triggers automatically when it doesn't matter */
  autoOrderTriggers: boolean;
  /** tap the creatures that crew a Vehicle when only one group of them makes sense (local only) */
  autoCrew: boolean;
  /** auto-pay also covers hybrid, two-or-color, snow and Phyrexian symbols (Phyrexian with mana only; local only) */
  autoPayHybrid: boolean;
  /** auto-pay also taps "any color" sources, last, for the color still owed (local only) */
  autoPayAnyColor: boolean;
  /** ...and sources that hurt (painlands, City of Brass) when nothing else can pay (local only) */
  autoPayPainful: boolean;
  /** auto-pay the cost of an X spell once X is announced (local only) */
  autoPayX: boolean;
  /** the X chooser starts at the most the player can pay (local only) */
  xDefaultsToMax: boolean;
  /** a menu with a single option picks it (local only) */
  skipSingleChoice: boolean;
  /** a choice among interchangeable cards or permanents (identical tokens) is made for the player (local only) */
  pickIdentical: boolean;
  /** "choose any number" prompts start with the obvious answers ticked, such as proliferate (local only) */
  smartPreselect: boolean;
  /** attackers attack the opponent unless dragged onto a planeswalker or battle (1v1, local only) */
  attackPlayerByDefault: boolean;
  /** combat damage is assigned without asking when only one split makes sense (local only) */
  autoAssignDamage: boolean;
  /** trample: lethal to the blockers and the rest to the player, without asking (local only) */
  autoAssignTrample: boolean;
  /** the hand is sorted: lands first, then by mana value and color (local only) */
  sortHand: boolean;
  /** shipped starter decks stay out of the library and deck pickers (local only) */
  hideStarterDecks: boolean;
  /** revealed cards fade away after a few seconds (local only) */
  autoDismissReveals: boolean;
  /** battlefield cards are drawn smaller, so big boards stay roomy (local only) */
  compactBoard: boolean;
  /** other players' quick emotes and emote bubbles stay hidden; your own still show (local only) */
  muteEmotes: boolean;
  /** pick the only legal target automatically (0 off, 1 when obvious, 2 always) */
  autoTargetLevel: 0 | 1 | 2;
  /** where the game stops to give you priority */
  stops: UserSkipPrioritySteps;
  /** combat steps where the client doesn't pass for you once creatures attack (local only) */
  combatStops: TurnCombatStops;
  /** card motion: placed, flown between zones */
  animations: boolean;
  /** opponents may ask to see your hand */
  allowHandRequests: boolean;
  /** pass priority for the player when they have nothing to cast, play or activate (local only) */
  autoPass: boolean;
  /** answer "no attacks" / "no blocks" when nothing can attack or block (local only) */
  autoSkipCombat: boolean;
  /** stop for permanents' activated abilities during the opponent's turn (local only) */
  abilitiesOnTheirTurn: boolean;
  /** don't take priority back after casting a spell or activating an ability: it resolves unless the opponent responds */
  passAfterCasting: boolean;
  /** game sounds (synthesized, local only); M toggles it during a game */
  sound: boolean;
  /** master volume, 0 to 1 */
  volume: number;
  /** only your turn, decisions, the clock and the result make a sound */
  importantCuesOnly: boolean;
  /** a short chime when a triggered ability goes on the stack (else the spell cue) */
  triggerSounds: boolean;
  /** browser notifications while the tab is in the background: your move, a draft pick, a game starting, a whisper */
  notifications: boolean;
  /** profile: the sigil shown for the player (server avatar ids 10 to 32; 10 shows the initial) */
  avatarId: number;
  /** profile: a country code ("de") or "world" */
  flag: string;
  /** the interface language, or "auto" for the browser's (local only) */
  locale: LocalePreference;
  /** playmat: the cloth the match is dyed in (local only) */
  matCloth: string;
  /** playmat: the art printed into your half: your deck's cover, a card you chose, or none (local only) */
  matArt: 'deck' | 'card' | 'none';
  /** playmat: the chosen card when matArt is 'card' */
  matCard: CardImageRef | null;
  /** watching: the game is shown this many seconds behind, so a stream of it can't give the players away (local only) */
  broadcastDelay: BroadcastDelay;
  /** watching: hands show as card backs, even ones the watcher may see (local only) */
  hideHands: boolean;
  /** watching and replays: the hovered card shows large at the side of the table, for stream viewers (local only) */
  largeZoom: boolean;
  /** Career: the player chose to try it (local only; a server Career profile counts too) */
  careerOptIn: boolean;
  /** Career: the menu music on Career's screens (synthesized, local only) */
  careerMusic: boolean;
  /** Career: the music's volume, 0 to 1, under the master volume (local only) */
  musicVolume: number;
  /** painted art (generated with an image model: icons, avatars, crests, sleeves, playmat prints) instead of the
   * drawn SVG and CSS art (local only) */
  paintedArt: boolean;
  /** full-art card styles from the capsule machines (ui/cardStyles.ts): everyone's, only mine, or none (local only) */
  cardStyles: 'all' | 'mine' | 'off';
  /** the back of hidden cards: one of Playmat's designs, or the original Magic back loaded from Scryfall like card
   * pictures (a Career sleeve's back still wins; local only) */
  cardBack: CardBackId;
}

const SETTINGS_KEY = 'playmat.settings';

/** Arena-like: the game moves on by itself whenever the player has no real decision. */
export const STREAMLINED: Partial<PlaySettings> = {
  autoPass: true, autoSkipCombat: true, abilitiesOnTheirTurn: false, passAfterCasting: true, autoPayMana: true, autoTargetLevel: 1, autoOrderTriggers: true,
  autoCrew: true, autoPayHybrid: true, autoPayAnyColor: true, autoPayX: true, xDefaultsToMax: true, skipSingleChoice: true, pickIdentical: true, smartPreselect: true, attackPlayerByDefault: true, autoAssignDamage: true,
};

/** Classic XMage: every stop and every choice is the player's. */
export const FULL_CONTROL: Partial<PlaySettings> = {
  autoPass: false, autoSkipCombat: false, abilitiesOnTheirTurn: true, passAfterCasting: false, autoPayMana: false, autoPayRestricted: true, autoTargetLevel: 0, autoOrderTriggers: false,
  autoCrew: false, autoPayHybrid: false, autoPayAnyColor: false, autoPayPainful: false, autoPayX: false, xDefaultsToMax: false, skipSingleChoice: false, pickIdentical: false, smartPreselect: false, attackPlayerByDefault: false, autoAssignDamage: false, autoAssignTrample: false,
};

export const DEFAULT_SETTINGS: PlaySettings = {
  autoPayMana: true,
  autoPayRestricted: true,
  confirmEmptyManaPool: true,
  autoOrderTriggers: true,
  autoCrew: true,
  autoPayHybrid: true,
  autoPayAnyColor: true,
  autoPayPainful: false,
  autoPayX: true,
  xDefaultsToMax: true,
  skipSingleChoice: true,
  pickIdentical: true,
  smartPreselect: true,
  attackPlayerByDefault: true,
  autoAssignDamage: true,
  autoAssignTrample: false,
  sortHand: true,
  hideStarterDecks: false,
  autoDismissReveals: true,
  compactBoard: false,
  muteEmotes: false,
  autoTargetLevel: 1,
  animations: true,
  allowHandRequests: true,
  autoPass: true,
  autoSkipCombat: true,
  abilitiesOnTheirTurn: false,
  passAfterCasting: true,
  sound: true,
  volume: 0.6,
  importantCuesOnly: false,
  triggerSounds: true,
  notifications: false,
  avatarId: 10,
  flag: 'world',
  locale: 'auto',
  matCloth: 'green',
  matArt: 'deck',
  matCard: null,
  broadcastDelay: 0,
  hideHands: true,
  largeZoom: false,
  careerOptIn: false,
  careerMusic: true,
  musicVolume: 0.5,
  paintedArt: true,
  cardStyles: 'all',
  cardBack: DEFAULT_CARD_BACK,
  stops: {
    yourTurn: { upkeep: false, draw: false, main1: true, beforeCombat: false, endOfCombat: false, main2: true, endOfTurn: false },
    opponentTurn: { upkeep: false, draw: false, main1: false, beforeCombat: false, endOfCombat: false, main2: false, endOfTurn: true },
    stopOnDeclareAttackers: true,
    stopOnDeclareBlockersWithZeroPermanents: false,
    stopOnDeclareBlockersWithAnyPermanents: true,
    stopOnAllMainPhases: true,
    stopOnAllEndPhases: true,
    stopOnStackNewObjects: true,
  },
  combatStops: {
    yourTurn: { attackers: false, blockers: false },
    opponentTurn: { attackers: false, blockers: false },
  },
};

interface SettingsState {
  settings: PlaySettings;
  /**
   * Arena's full control, for this visit: the game stops at every step, nothing is passed for the player and priority
   * comes back after each spell. Not saved, like on Arena.
   */
  fullControl: boolean;
  setFullControl(on: boolean): void;
  update(patch: Partial<PlaySettings>): void;
  syncToServer(): Promise<void>;
}

/** The complete preference object the server keeps per user (every field is required server side). */
export function toUserData(settings: PlaySettings, fullControl = false): UserData {
  return {
    groupId: 0,
    avatarId: settings.avatarId,
    allowRequestShowHandCards: settings.allowHandRequests,
    userSkipPrioritySteps: serverStops(settings.stops, fullControl),
    flagName: `${settings.flag || 'world'}.png`,
    askMoveToGraveOrder: false,
    manaPoolAutomatic: !fullControl && (settings.autoPayMana || !settings.autoPayRestricted),
    manaPoolAutomaticRestricted: settings.autoPayRestricted,
    passPriorityCast: settings.passAfterCasting && !fullControl,
    passPriorityActivation: settings.passAfterCasting && !fullControl,
    autoOrderTrigger: settings.autoOrderTriggers,
    autoTargetLevel: settings.autoTargetLevel,
    useSameSettingsForReplacementEffects: true,
    useFirstManaAbility: false,
    confirmEmptyManaPool: settings.confirmEmptyManaPool,
    userIdStr: '',
  };
}

export const useSettings = create<SettingsState>((set, get) => ({
  settings: { ...DEFAULT_SETTINGS, ...readJson<Partial<PlaySettings>>(SETTINGS_KEY, {}) },
  fullControl: false,

  setFullControl(on) {
    if (get().fullControl === on) return;
    set({ fullControl: on });
    void get().syncToServer();
  },

  update(patch) {
    const settings = { ...get().settings, ...patch };
    writeJson(SETTINGS_KEY, settings);
    set({ settings });
    void get().syncToServer();
  },

  async syncToServer() {
    if (useSession.getState().phase !== 'signedIn') return;
    await api.connectSetUserData(null, toUserData(get().settings, get().fullControl), 'web', '').catch(() => undefined);
  },
}));

// the server forgets preferences with the session: send them after every login, the silent one after a dropped
// connection included (a tablet waking up). Without them the server plays by its defaults, which skip the beginning
// of combat: a Vehicle Greasefang returns could not be crewed.
useSession.subscribe((state, previous) => {
  if (state.phase === 'signedIn' && state.loginCount !== previous.loginCount) {
    void useSettings.getState().syncToServer();
  }
});

const AUTOMATION_KEYS = [
  'autoCrew', 'autoPayHybrid', 'autoPayAnyColor', 'autoPayPainful', 'autoPayX', 'xDefaultsToMax', 'skipSingleChoice',
  'pickIdentical', 'smartPreselect', 'attackPlayerByDefault', 'autoAssignDamage', 'autoAssignTrample',
] as const;

/** The Settings → Play automations that answer or pre-fill for the player. */
export type Automation = Pick<PlaySettings, (typeof AUTOMATION_KEYS)[number]>;

/** The automations a match runs now: the player's toggles, every one off while full control is locked on. */
export function useAutomation(): Automation {
  return useSettings(useShallow((state) => Object.fromEntries(
    AUTOMATION_KEYS.map((key) => [key, state.settings[key] && !state.fullControl]),
  ) as unknown as Automation));
}
