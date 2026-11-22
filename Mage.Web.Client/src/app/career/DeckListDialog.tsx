import { useQuery } from '@tanstack/react-query';
import { Check, Save } from 'lucide-react';
import { useMemo, useState } from 'react';
import type { DeckCardInfo } from '../../protocol/generated/views';
import type { DeckCardLists } from '../../core/decks/types';
import { api } from '../connection';
import { useT } from '../i18n';
import { notify } from '../stores/toasts';
import { Button } from '../ui/Button';
import { Dialog } from '../ui/Dialog';
import { saveNamedCareerDeck, useCareerCollection, useCareerDecks } from './careerData';
import { ownedByName, shortfalls } from './careerModel';
import { mergeByName } from './careerModesModel';
import styles from './CareerModes.module.css';

function rows(cards: readonly DeckCardInfo[] | undefined) {
  return mergeByName((cards ?? []).map((card) => ({ name: card.cardName, amount: card.amount }))).sort((a, b) => a.name.localeCompare(b.name));
}

function toDeckCardLists(name: string, cards: readonly DeckCardInfo[] | undefined, sideboard: readonly DeckCardInfo[] | undefined): DeckCardLists {
  const map = (list: readonly DeckCardInfo[] | undefined) => (list ?? []).map((entry) => ({
    cardName: entry.cardName ?? '',
    setCode: entry.setCode ?? '',
    cardNumber: entry.cardNumber ?? '',
    amount: entry.amount ?? 1,
  }));
  return { name, cards: map(cards), sideboard: map(sideboard) };
}

/** The list of the deck a campaign duel hands you, to study before you sit down — or save to Career decks once owned. */
export function DeckListDialog({ campaignId, chapterId, nodeId, name, onClose }: {
  campaignId: string;
  chapterId?: string;
  nodeId: string;
  name: string;
  onClose(): void;
}) {
  const t = useT();
  const deck = useQuery({ queryKey: ['career', 'campaignDeck', campaignId, nodeId], queryFn: () => api.careerCampaignDeck(campaignId, nodeId), staleTime: Infinity });
  const collection = useCareerCollection(true);
  const savedDecks = useCareerDecks(true);
  const [saving, setSaving] = useState(false);

  const main = rows(deck.data?.cards);
  const side = rows(deck.data?.sideboard);
  const count = main.reduce((sum, row) => sum + row.amount, 0);
  // a Commander or Brawl deck keeps its commander in the sideboard
  const commander = side.length > 0 && side.length <= 2 && (count === 99 || count === 98 || count === 59 || count === 58);

  const owned = useMemo(() => ownedByName(collection.data ?? []), [collection.data]);
  const asDeck = useMemo(() => (deck.data ? toDeckCardLists(name, deck.data.cards, deck.data.sideboard) : null), [deck.data, name]);
  const ownsAll = useMemo(() => (asDeck && collection.data ? shortfalls(asDeck, owned).length === 0 : false), [asDeck, collection.data, owned]);
  const source = chapterId ? `campaign:${campaignId}/${chapterId}` : null;
  const alreadySaved = useMemo(() => !!source && (savedDecks.data ?? []).some((item) => item.source === source), [savedDecks.data, source]);

  async function saveCopy() {
    if (!asDeck) return;
    setSaving(true);
    try {
      await saveNamedCareerDeck(null, name, asDeck, false);
      notify(t('career.decks.savedToast'), t('career.decks.savedDetail', { name }));
    } catch (error) {
      notify(t('career.decks.saveFailed'), error instanceof Error ? error.message : String(error), 'error');
    } finally {
      setSaving(false);
    }
  }

  return (
    <Dialog
      open
      onOpenChange={(open) => !open && onClose()}
      title={name}
      description={t('career.deckList.count', { count })}
      width="md"
      footer={ownsAll ? (
        <>
          {alreadySaved && <span className={styles.small}><Check size={14} aria-hidden="true" /> {t('career.decks.alreadySaved')}</span>}
          <Button variant={alreadySaved ? 'quiet' : 'print'} size="sm" icon={<Save size={15} />} busy={saving} onClick={() => void saveCopy()}>
            {t('career.decks.saveToCareer')}
          </Button>
        </>
      ) : undefined}
    >
      {deck.isPending && <p className={styles.note}>{t('career.loading')}</p>}
      {deck.isError && <p className={styles.error} role="alert">{t('career.failed')}</p>}
      {deck.data && (
        <div className={styles.deckDialog}>
          {side.length > 0 && commander && (
            <section>
              <h4 className={styles.subhead}>{t('career.deckList.commander')}</h4>
              <ul>{side.map((row) => <li key={row.name}>{row.name}</li>)}</ul>
            </section>
          )}
          <section>
            <h4 className={styles.subhead}>{t('career.deckList.main')}</h4>
            <ul>{main.map((row) => <li key={row.name}><b>{row.amount}</b> {row.name}</li>)}</ul>
          </section>
          {side.length > 0 && !commander && (
            <section>
              <h4 className={styles.subhead}>{t('career.deckList.sideboard')}</h4>
              <ul>{side.map((row) => <li key={row.name}><b>{row.amount}</b> {row.name}</li>)}</ul>
            </section>
          )}
        </div>
      )}
    </Dialog>
  );
}
