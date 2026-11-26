import { afterEach, expect, test, vi } from 'vitest';
import { loadTestCatalog } from '../../test/catalog';
import { useCareerLook } from '../career/careerData';
import { api } from '../connection';
import { queryClient } from '../queries';
import { useDecks } from '../stores/decks';
import { readJson } from '../stores/persist';
import { usePlay } from '../stores/play';
import { useSession } from '../stores/session';

afterEach(() => {
  vi.clearAllTimers();
  queryClient.clear();
  vi.useRealTimers();
  vi.restoreAllMocks();
});

test('a browser with no Career look keeps the pieces the server holds instead of taking them off', async () => {
  vi.useFakeTimers();
  loadTestCatalog();
  queryClient.clear();
  localStorage.removeItem('playmat.career.look');
  useSession.setState({ phase: 'signedIn', userName: 'Alice' });
  usePlay.setState({ deckId: null });
  useDecks.setState({ selectedId: null, sleeves: {}, starters: [] });
  vi.spyOn(api, 'serverInfo').mockResolvedValue({ accounts: true, career: true });
  vi.spyOn(api, 'careerCosmetics').mockResolvedValue([]);
  vi.spyOn(api, 'supportPerks').mockResolvedValue({ perks: [{ kind: 'title', item: 'patron' }, { kind: 'pet', item: 'party-corgi' }] });
  // what another device sent: Career pieces and the thank-you set
  vi.spyOn(api, 'profileLooks').mockResolvedValue([{ name: 'Alice', avatar: 'owl', frame: 'frame-bronze', title: 'patron', pet: 'party-corgi', arrow: 'arrow-ember', badges: ['first-win'] }]);
  vi.spyOn(api, 'profileSetLook').mockImplementation(async (look) => ({ ...look, name: 'Alice' }));

  await import('./lookSync');
  await vi.advanceTimersByTimeAsync(1500);

  const sent = vi.mocked(api.profileSetLook).mock.calls.at(-1)?.[0];
  expect(sent).toMatchObject({ avatar: 'owl', frame: 'frame-bronze', arrow: 'arrow-ember', badges: ['first-win'], title: 'patron', pet: 'party-corgi' });
  // the Career pieces are this browser's now; the thank-you ones stay with the thank-you set
  const adopted = { avatar: 'owl', title: null, frame: 'frame-bronze', arrow: 'arrow-ember', badges: ['first-win'] };
  expect(readJson<Record<string, unknown>>('playmat.career.look', {}).alice).toEqual(adopted);
  expect(useCareerLook.getState().looks.alice).toEqual(adopted);
});

test('the server look adds nothing when the browser has its own Career look', async () => {
  const { careerLookFrom } = await import('./lookSync');
  expect(careerLookFrom({ title: 'patron', frame: 'supporter-ring' }, [{ kind: 'title', item: 'patron' }, { kind: 'frame', item: 'supporter-ring' }])).toBeNull();
  expect(careerLookFrom({ title: 'title-duelist', lands: 'lands-snow' }, [])).toEqual({ avatar: null, title: 'title-duelist', lands: 'lands-snow' });
});
