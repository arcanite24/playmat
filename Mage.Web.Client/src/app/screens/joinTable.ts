import { api } from '../connection';
import { toWire } from '../decks/deckModel';
import { refreshTables } from '../queries';
import { useDecks } from '../stores/decks';
import { useSession } from '../stores/session';
import { warmDeck } from '../ui/imageCache';
import type { TableView } from '../../protocol/generated/views';

/** Takes a seat at a table with one of the player's decks. Throws with a message the player can act on. */
export async function joinTable(table: TableView, deckId: string, password = ''): Promise<void> {
  const { roomId, userName } = useSession.getState();
  if (!roomId || !table.tableId) throw new Error('Not connected to the server.');
  const { deck } = await useDecks.getState().loadForPlay(deckId);
  // the pictures download while the table fills, so the first hand lands with its art
  warmDeck(deck);
  const joined = await api.roomJoinTable(roomId, table.tableId, userName, 'HUMAN', 1, toWire(deck), password);
  refreshTables();
  if (!joined) {
    throw new Error(table.passworded ? 'Wrong password, or the table did not accept your deck.' : 'The table did not accept your seat or deck.');
  }
}
