import { afterEach, expect, test, vi } from 'vitest';
import type { CareerCatalogItem } from '../../protocol/generated/views';
import { loadTestCatalog } from '../../test/catalog';
import { api } from '../connection';
import { queryClient } from '../queries';
import { useDecks } from '../stores/decks';
import { writeJson } from '../stores/persist';
import { usePlay } from '../stores/play';
import { useSession } from '../stores/session';
import { useSettings } from '../stores/settings';
import { setCatalog } from '../ui/cosmeticsCatalog';
import { useLooks } from './supportData';

afterEach(() => {
  vi.clearAllTimers();
  queryClient.clear();
  vi.useRealTimers();
  vi.restoreAllMocks();
});

test('a cold sign-in waits for the catalog before uploading the equipped cosmetics', async () => {
  vi.useFakeTimers();
  const catalog = loadTestCatalog();
  setCatalog([]);
  queryClient.clear();
  useSession.setState({ phase: 'signedIn', userName: 'Alice' });
  usePlay.setState({ deckId: null });
  useDecks.setState({
    selectedId: 'starter:test', sleeves: { 'starter:test': '#6e3f24' },
    starters: [{ file: 'test', name: 'Test', cards: 60, cover: { name: 'Forest', setCode: 'M21', cardNumber: '274' } }],
  });
  useSettings.getState().update({ matCloth: 'sand' });
  writeJson('playmat.career.look', { alice: { avatar: 'owl', frame: 'frame-bronze' } });
  vi.spyOn(api, 'serverInfo').mockResolvedValue({ accounts: true, career: true });
  let resolveCatalog!: (items: CareerCatalogItem[]) => void;
  vi.spyOn(api, 'careerCosmetics').mockImplementation(() => new Promise((resolve) => { resolveCatalog = resolve; }));
  vi.spyOn(api, 'supportPerks').mockResolvedValue({ perks: [] });
  vi.spyOn(api, 'profileSetLook').mockImplementation(async (look) => ({ ...look, name: 'Alice' }));

  await import('./lookSync');
  await vi.advanceTimersByTimeAsync(1500);
  expect(api.careerCosmetics).toHaveBeenCalledOnce();
  expect(api.profileSetLook).not.toHaveBeenCalled();
  resolveCatalog(catalog);
  await vi.advanceTimersByTimeAsync(0);
  const look = { avatar: 'owl', frame: 'frame-bronze', sleeve: 'copper', cloth: 'sand' };
  expect(api.profileSetLook).toHaveBeenCalledWith(look);
  expect(useLooks.getState().looks.alice).toEqual({ ...look, name: 'Alice' });
});
