import type { CareerPullResult } from '../../../protocol/generated/views';
import { readJson, writeJson } from '../../stores/persist';

/**
 * Capsules pulled but not opened yet (B328): the server gave them the moment the pull went through, so closing the
 * sequence early (or losing the page) must not lose the opening. The last pull waits here, per account, until its
 * sequence finishes; the shop offers to open it and keeps its capsules off the board meanwhile.
 */
const KEY = 'playmat.capsules.unopened';

export function readUnopened(account: string): { result: CareerPullResult } | null {
  const saved = readJson<Record<string, CareerPullResult>>(KEY, {})[account];
  return saved && Array.isArray(saved.pulls) && saved.pulls.length > 0 ? { result: saved } : null;
}

export function saveUnopened(account: string, result: CareerPullResult | null): void {
  const all = readJson<Record<string, CareerPullResult>>(KEY, {});
  if (result) all[account] = result;
  else delete all[account];
  writeJson(KEY, all);
}
