import type { CallbackMethodName, CallbackPayloads } from '../../protocol/generated/protocol';
import type {
  AbilityPickerView,
  CardView,
  ChoiceImpl,
  GameClientMessage,
  GameView,
  MultiAmountMessage,
} from '../../protocol/generated/views';

/**
 * A question from the server that the player must answer. Parsed from the dialog events
 * (GAME_SELECT, GAME_TARGET, GAME_ASK, ...) without relying on English message text.
 */
export type Prompt =
  | PriorityPrompt
  | DeclareAttackersPrompt
  | DeclareBlockersPrompt
  | TargetPrompt
  | AskPrompt
  | ChooseAbilityPrompt
  | ChoosePilePrompt
  | ChooseChoicePrompt
  | PlayManaPrompt
  | AmountPrompt
  | MultiAmountPrompt;

/**
 * Questions asked before the first turn, named by the server bridge (`options.webPrompt`, see docs/WebSocketAPI.md
 * "Pre-game prompt markers") so they need no guessing from the text.
 */
export type PregameMarker = 'mulligan' | 'mulliganBottom' | 'startingPlayer';

const PREGAME_MARKERS: readonly PregameMarker[] = ['mulligan', 'mulliganBottom', 'startingPlayer'];

interface PromptBase {
  /** server message, may contain simple HTML (colors) */
  message: string;
  /** message without markup */
  text: string;
  options: Record<string, unknown>;
  /** the pre-game question this is, when the server says so */
  marker?: PregameMarker | null;
}

export interface PriorityPrompt extends PromptBase {
  kind: 'priority';
}

export interface DeclareAttackersPrompt extends PromptBase {
  kind: 'declareAttackers';
  possibleAttackers: string[];
  /** label of the "attack with everything" button, when offered */
  allAttackLabel: string | null;
}

export interface DeclareBlockersPrompt extends PromptBase {
  kind: 'declareBlockers';
  possibleBlockers: string[];
}

export interface TargetPrompt extends PromptBase {
  kind: 'target';
  /** valid ids: permanents, players, stack objects or cards */
  targets: string[];
  /** cards that are not on the board (library, graveyard search...) and must be shown to choose from */
  cards: Record<string, CardView> | null;
  required: boolean;
  /** already chosen targets of this choice */
  chosen: string[];
  /** label for finishing the choice when enough targets are chosen */
  doneLabel: string | null;
}

export interface AskPrompt extends PromptBase {
  kind: 'ask';
  /** answer true */
  yesLabel: string;
  /** answer false */
  noLabel: string;
  isMulligan: boolean;
}

export interface ChooseAbilityPrompt extends PromptBase {
  kind: 'chooseAbility';
  /** ability id -> rule text, in server order */
  choices: { id: string; text: string }[];
}

export interface ChoosePilePrompt extends PromptBase {
  kind: 'choosePile';
  pile1: CardView[];
  pile2: CardView[];
}

export interface ChooseChoicePrompt extends PromptBase {
  kind: 'chooseChoice';
  choice: ChoiceImpl;
}

export interface PlayManaPrompt extends PromptBase {
  kind: 'playMana';
  /** label of a special payment (convoke, delve, improvise), when offered */
  specialLabel: string | null;
  isX: boolean;
}

export interface AmountPrompt extends PromptBase {
  kind: 'amount';
  min: number;
  max: number;
}

export interface MultiAmountPrompt extends PromptBase {
  kind: 'multiAmount';
  items: MultiAmountMessage[];
  min: number;
  max: number;
  title: string | null;
  header: string | null;
  canCancel: boolean;
}

export type PromptEventName =
  | 'GAME_SELECT' | 'GAME_TARGET' | 'GAME_ASK' | 'GAME_CHOOSE_ABILITY' | 'GAME_CHOOSE_PILE'
  | 'GAME_CHOOSE_CHOICE' | 'GAME_PLAY_MANA' | 'GAME_PLAY_XMANA' | 'GAME_GET_AMOUNT' | 'GAME_GET_MULTI_AMOUNT';

export const PROMPT_EVENTS: readonly PromptEventName[] = [
  'GAME_SELECT', 'GAME_TARGET', 'GAME_ASK', 'GAME_CHOOSE_ABILITY', 'GAME_CHOOSE_PILE',
  'GAME_CHOOSE_CHOICE', 'GAME_PLAY_MANA', 'GAME_PLAY_XMANA', 'GAME_GET_AMOUNT', 'GAME_GET_MULTI_AMOUNT',
];

export function isPromptEvent(method: CallbackMethodName): method is PromptEventName {
  return (PROMPT_EVENTS as readonly string[]).includes(method);
}

export function stripMarkup(html: string | undefined): string {
  if (!html) return '';
  return html
    .replace(/<br\s*\/?>/gi, ' ')
    .replace(/<[^>]*>/g, '')
    .replace(/&nbsp;/g, ' ')
    .replace(/&mdash;/g, '—')
    .replace(/&ndash;/g, '–')
    .replace(/&minus;/g, '−')
    .replace(/&bull;/g, '•')
    .replace(/&hellip;/g, '…')
    .replace(/&[lr]squo;/g, "'")
    .replace(/&[lr]dquo;/g, '"')
    .replace(/&#(\d+);/g, (_, code: string) => String.fromCodePoint(Number(code)))
    .replace(/&amp;/g, '&')
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'")
    .replace(/\s+/g, ' ')
    .trim();
}

/** The server's pre-game marker in a prompt's options, if any. */
export function pregameMarker(options: Record<string, unknown> | null | undefined): PregameMarker | null {
  const value = options?.webPrompt;
  return typeof value === 'string' && (PREGAME_MARKERS as readonly string[]).includes(value) ? value as PregameMarker : null;
}

function base(message: GameClientMessage | null | undefined): PromptBase {
  const raw = message?.message ?? '';
  const options = (message?.options ?? {}) as Record<string, unknown>;
  return {
    message: raw,
    text: stripMarkup(raw),
    options,
    marker: pregameMarker(options),
  };
}

function idList(value: unknown): string[] {
  if (Array.isArray(value)) return value.filter((id): id is string => typeof id === 'string');
  return [];
}

function label(value: unknown): string | null {
  return typeof value === 'string' && value.trim() ? value : null;
}

/** Turns a dialog event payload into a prompt. */
export function parsePrompt<M extends PromptEventName>(method: M, data: CallbackPayloads[M]): Prompt {
  switch (method) {
    case 'GAME_SELECT': {
      const message = data as GameClientMessage;
      const common = base(message);
      if ('possibleAttackers' in common.options) {
        return {
          ...common,
          kind: 'declareAttackers',
          possibleAttackers: idList(common.options.possibleAttackers),
          allAttackLabel: label(common.options.specialButton),
        };
      }
      if ('possibleBlockers' in common.options) {
        return { ...common, kind: 'declareBlockers', possibleBlockers: idList(common.options.possibleBlockers) };
      }
      // the priority prompt is the only select without options
      return { ...common, kind: 'priority' };
    }
    case 'GAME_TARGET': {
      const message = data as GameClientMessage;
      const common = base(message);
      const cards = message.cardsView1 && Object.keys(message.cardsView1).length > 0 ? message.cardsView1 : null;
      return {
        ...common,
        kind: 'target',
        targets: idList(message.targets).length > 0
          ? idList(message.targets)
          : idList(common.options.possibleTargets).length > 0
            ? idList(common.options.possibleTargets)
            : Object.keys(cards ?? {}),
        cards,
        required: message.flag === true,
        chosen: idList(common.options.chosenTargets),
        doneLabel: label(common.options['UI.right.btn.text']),
      };
    }
    case 'GAME_ASK': {
      const common = base(data as GameClientMessage);
      const yesLabel = label(common.options['UI.left.btn.text']) ?? 'Yes';
      return {
        ...common,
        kind: 'ask',
        yesLabel,
        noLabel: label(common.options['UI.right.btn.text']) ?? 'No',
        // servers without the bridge marker: the mulligan question is the one whose "yes" is "Mulligan"
        isMulligan: common.marker ? common.marker === 'mulligan' : yesLabel === 'Mulligan',
      };
    }
    case 'GAME_CHOOSE_ABILITY': {
      const view = data as AbilityPickerView;
      return {
        message: view.message ?? '',
        text: stripMarkup(view.message),
        options: {},
        kind: 'chooseAbility',
        choices: Object.entries(view.choices ?? {}).map(([id, text]) => ({ id, text })),
      };
    }
    case 'GAME_CHOOSE_PILE': {
      const message = data as GameClientMessage;
      return {
        ...base(message),
        kind: 'choosePile',
        pile1: Object.values(message.cardsView1 ?? {}),
        pile2: Object.values(message.cardsView2 ?? {}),
      };
    }
    case 'GAME_CHOOSE_CHOICE': {
      const message = data as GameClientMessage;
      // the server sends ChoiceImpl for the Choice interface
      return { ...base(message), kind: 'chooseChoice', choice: (message.choice ?? {}) as ChoiceImpl };
    }
    case 'GAME_PLAY_MANA':
    case 'GAME_PLAY_XMANA': {
      const message = data as GameClientMessage;
      const common = base(message);
      return {
        ...common,
        kind: 'playMana',
        specialLabel: label(common.options.specialButton) ?? (message?.gameView?.special ? specialPaymentLabel(message.gameView) : null),
        isX: method === 'GAME_PLAY_XMANA',
      };
    }
    case 'GAME_GET_AMOUNT': {
      const message = data as GameClientMessage;
      return { ...base(message), kind: 'amount', min: message.min ?? 0, max: message.max ?? 0 };
    }
    case 'GAME_GET_MULTI_AMOUNT': {
      const message = data as GameClientMessage;
      const common = base(message);
      return {
        ...common,
        kind: 'multiAmount',
        items: message.messages ?? [],
        min: message.min ?? 0,
        max: message.max ?? 0,
        title: label(common.options.title),
        header: label(common.options.header),
        canCancel: common.options.canCancel === true,
      };
    }
    default:
      throw new Error(`Not a prompt event: ${String(method)}`);
  }
}

const SPECIAL_PAYMENTS = ['Delve', 'Convoke', 'Improvise', 'Waterbend'];

/**
 * The server offers a special way to pay (delve, convoke, improvise) only through the view's `special` flag: name the
 * button after the keyword of the spell on the stack that has one.
 */
function specialPaymentLabel(view: GameView): string {
  for (const item of Object.values(view.stack ?? {}).reverse()) {
    const rules = (item.rules ?? []).join(' ').replace(/<[^>]*>/g, '');
    const keyword = SPECIAL_PAYMENTS.find((name) => new RegExp(`\\b${name}\\b`, 'i').test(rules));
    if (keyword) return keyword;
  }
  return 'Special payment';
}

/** The game view carried by a prompt event, if any. */
export function promptGameView(method: PromptEventName, data: unknown) {
  if (method === 'GAME_CHOOSE_ABILITY') return (data as AbilityPickerView).gameView;
  return (data as GameClientMessage | null)?.gameView;
}

/**
 * What the server's ability chooser is for ("Choose spell or ability to play Evolving Wilds [a16]" -> "Evolving
 * Wilds"), or null when the question is another one (modes, triggers).
 */
export function abilityChooserSource(text: string): string | null {
  const name = /^choose (?:spell or ability|ability) to (?:play|activate|cast)(?: for free)? (.+)$/i.exec(text.trim())?.[1];
  return name ? name.replace(/\s*\[[0-9a-f]{3,}\]/gi, '').trim() || null : null;
}
