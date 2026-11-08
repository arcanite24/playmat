import { create } from 'zustand';
import type { TokenChoices } from '../ui/tokenArt';
import { readJson, writeJson } from './persist';
import { useSession } from './session';

/**
 * The account's token art choices (ui/tokenArt.ts). Kept in this browser per account, and on servers with accounts
 * also in the account's look (support/lookSync.ts), so they follow the player to another device and the table sees
 * them. A choice made here waits as `dirty` until the server has it; a clean browser takes the server's choices when
 * signing in.
 */

const KEY = 'playmat.tokenArt';

interface Book {
  [account: string]: { choices: TokenChoices; dirty?: boolean };
}

/** Two sets of choices are the same whatever order their keys came in. */
export function sameChoices(a: TokenChoices | null | undefined, b: TokenChoices | null | undefined): boolean {
  const left = Object.entries(a ?? {});
  return left.length === Object.keys(b ?? {}).length && left.every(([name, picture]) => b?.[name] === picture);
}

function accountKey(): string {
  const { serverUrl, userName } = useSession.getState();
  return `${serverUrl}|${userName.toLowerCase()}`;
}

function read(account: string) {
  return readJson<Book>(KEY, {})[account] ?? { choices: {} };
}

function write(account: string, entry: { choices: TokenChoices; dirty?: boolean }) {
  writeJson(KEY, { ...readJson<Book>(KEY, {}), [account]: entry });
}

interface TokenArtState {
  account: string;
  choices: TokenChoices;
  /** changed here and not yet kept by the server */
  dirty: boolean;
  /** choose a picture for a token, or null for its default */
  choose(name: string, picture: string | null): void;
  /** the server's choices, taken when this browser has nothing waiting to go up */
  adopt(choices: TokenChoices | null | undefined): void;
  /** the server kept what was sent */
  saved(choices: TokenChoices): void;
}

const initial = accountKey();

export const useTokenArt = create<TokenArtState>((set, get) => ({
  account: initial,
  choices: read(initial).choices,
  dirty: !!read(initial).dirty,
  choose(name, picture) {
    const choices = { ...get().choices };
    if (picture) choices[name] = picture;
    else delete choices[name];
    write(get().account, { choices, dirty: true });
    set({ choices, dirty: true });
  },
  adopt(choices) {
    if (get().dirty) return;
    const next = { ...(choices ?? {}) };
    write(get().account, { choices: next });
    set({ choices: next });
  },
  saved(choices) {
    // a choice made while the look was on its way stays waiting
    if (!sameChoices(choices, get().choices)) return;
    write(get().account, { choices });
    set({ dirty: false });
  },
}));

// another account on this browser has its own choices
useSession.subscribe((state, previous) => {
  if (state.userName === previous.userName && state.serverUrl === previous.serverUrl) return;
  const account = accountKey();
  const entry = read(account);
  useTokenArt.setState({ account, choices: entry.choices, dirty: !!entry.dirty });
});
