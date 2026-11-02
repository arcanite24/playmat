import { useQuery } from '@tanstack/react-query';
import { useCallback, useMemo, useRef, useState } from 'react';
import {
  fixCard, issuesOf, leaveOut, readList, startDraft, type DeckReading, type ImportDraft,
} from '../../../core/deckImport/draft';
import { decodePayload } from '../../../core/deckImport/payload';
import { recognize, type Recognized } from '../../../core/deckImport/recognize';
import { describeFailure, readingFromServer, type ImportFailure } from '../../../core/deckImport/remote';
import { browserFixStore, importKey, resolveCards, type CardLookup, type PrintingPolicy } from '../../../core/deckImport/resolve';
import { scryfallRenamedCard } from '../../../core/deckImport/scryfallNames';
import { configuredImageProxy, proxiedScryfallUrl } from '../../../core/images/imageProxy';
import { nameFromLink, siteById, type DeckSiteId } from '../../../core/deckImport/sites';
import type { CardView } from '../../../protocol/generated/views';
import { APP_NAME } from '../../brand';
import { api } from '../../connection';
import type { ImportSeed } from '../../stores/importSheet';

export const cardLookup: CardLookup = {
  lookupCards: (cards) => api.lookupCards(cards),
  searchCards: (criteria) => api.searchCards(criteria),
  renamedCard: (name) => scryfallRenamedCard(name, async (url) => {
    const response = await fetch(proxiedScryfallUrl(url, configuredImageProxy()), { headers: { Accept: 'application/json' } });
    if (!response.ok) throw new Error(`Scryfall answered ${response.status}`);
    return response.json();
  }),
};

export type ImportStep =
  | { kind: 'input'; failure?: string }
  | { kind: 'loading'; site?: DeckSiteId }
  | { kind: 'assisted'; site: DeckSiteId; url: string; remoteId: string; failure?: ImportFailure }
  | { kind: 'preview'; draft: ImportDraft };

/** Which direct sites the server can read right now (a degraded one goes to the copy-and-paste flow). */
export function useDegradedSites(): ReadonlySet<string> {
  const sources = useQuery({
    queryKey: ['deckImportSources'],
    queryFn: () => api.deckImportSources(),
    staleTime: 5 * 60_000,
    retry: false,
  });
  return useMemo(() => new Set((sources.data ?? []).filter((source) => source.status === 'degraded').map((source) => source.site ?? '')), [sources.data]);
}

const NO_CARDS = 'No cards found. Paste one card per line, like “4 Lightning Bolt”, or a link to a deck.';

/** The import sheet's steps: read the input, fetch or guide, then a draft the player fixes and saves. */
export function useImportFlow() {
  const [text, setText] = useState('');
  const [step, setStep] = useState<ImportStep>({ kind: 'input' });
  const degraded = useDegradedSites();
  const run = useRef(0);
  const fixes = useMemo(() => browserFixStore(), []);

  const recognized: Recognized = useMemo(() => recognize(text), [text]);

  const draftFrom = useCallback(async (reading: DeckReading, fallbackName?: string, printings: PrintingPolicy = 'site') => {
    const ticket = ++run.current;
    setStep({ kind: 'loading', site: reading.source?.site });
    try {
      if (reading.list.main.length + reading.list.side.length + reading.list.commanders.length + reading.list.companion.length === 0) {
        if (ticket === run.current) setStep({ kind: 'input', failure: NO_CARDS });
        return;
      }
      const draft = await startDraft(reading, cardLookup, { fixes, printings, fallbackName });
      if (ticket === run.current) setStep({ kind: 'preview', draft });
    } catch {
      if (ticket === run.current) setStep({ kind: 'input', failure: 'Couldn’t reach the card database. Check the connection and try again.' });
    }
  }, [fixes]);

  /** Read what's in the field: a list, a direct deck link, or a link to a site that needs copy and paste. */
  const submit = useCallback(async (input: string = text) => {
    const found = recognize(input);
    if (found.kind === 'empty') {
      setStep({ kind: 'input', failure: input.trim() ? NO_CARDS : undefined });
      return;
    }
    if (found.kind === 'unknownUrl') {
      setStep({ kind: 'input', failure: `${APP_NAME} can’t read decks from that site yet. Export the deck there and paste its list here.` });
      return;
    }
    if (found.kind === 'list') {
      await draftFrom(readList(found.text));
      return;
    }
    const site = siteById(found.site)!;
    if (site.tier === 'assisted' || degraded.has(site.id)) {
      setStep({ kind: 'assisted', site: site.id, url: found.url, remoteId: found.remoteId });
      return;
    }
    const ticket = ++run.current;
    setStep({ kind: 'loading', site: site.id });
    let reading: DeckReading;
    try {
      reading = readingFromServer(await api.deckImportFromUrl(found.url), site.id, found.url);
    } catch (error) {
      if (ticket !== run.current) return;
      const failure = describeFailure(error, site.id);
      setStep(failure.assisted
        ? { kind: 'assisted', site: site.id, url: found.url, remoteId: found.remoteId, failure }
        : { kind: 'input', failure: failure.message });
      return;
    }
    if (ticket !== run.current) return;
    await draftFrom(reading, nameFromLink(found.url) ?? `${site.name} deck`);
  }, [text, degraded, draftFrom]);

  /** The list the player copied from a site that blocks us, joined to the link they gave. */
  const finishAssisted = useCallback(async (pasted: string) => {
    if (step.kind !== 'assisted') return false;
    const found = recognize(pasted);
    if (found.kind !== 'list') return false;
    const reading = readList(found.text);
    await draftFrom({
      ...reading,
      source: { site: step.site, url: step.url, remoteId: step.remoteId },
    }, nameFromLink(step.url) ?? `${siteById(step.site)?.name ?? 'Imported'} deck`);
    return true;
  }, [step, draftFrom]);

  /** Start from what the sheet was opened with. */
  const seed = useCallback((value: ImportSeed | null) => {
    run.current += 1;
    if (!value) {
      setText('');
      setStep({ kind: 'input' });
      return;
    }
    if (value.kind === 'text') {
      setText(value.text);
      void submit(value.text);
      return;
    }
    if (value.kind === 'file') {
      setText('');
      void draftFrom(readList(value.text), value.name.replace(/\.[^.]+$/, ''));
      return;
    }
    const payload = value.payload;
    const found = recognize(payload.url);
    const site = siteById(payload.site);
    const reading = readList(payload.text);
    setText('');
    void draftFrom({
      ...reading,
      name: payload.name ?? reading.name,
      source: site && found.kind === 'site' ? { site: site.id, url: found.url, remoteId: found.remoteId } : undefined,
    }, nameFromLink(payload.url) ?? `${site?.name ?? 'Imported'} deck`);
  }, [submit, draftFrom]);

  const cancel = useCallback(() => {
    run.current += 1;
    setStep({ kind: 'input' });
  }, []);

  const backToInput = useCallback(() => {
    run.current += 1;
    setStep({ kind: 'input' });
  }, []);

  const updateDraft = useCallback((update: (draft: ImportDraft) => ImportDraft) => {
    setStep((current) => (current.kind === 'preview' ? { kind: 'preview', draft: update(current.draft) } : current));
  }, []);

  const pick = useCallback((key: string, card: CardView, original: { cardName: string; setCode: string | null; cardNumber: string | null }) => {
    fixes.set(importKey(original), { cardName: card.name ?? original.cardName, setCode: card.expansionSetCode ?? '', cardNumber: card.cardNumber ?? '' });
    updateDraft((draft) => fixCard(draft, key, card));
  }, [fixes, updateDraft]);

  const skip = useCallback((key: string) => updateDraft((draft) => leaveOut(draft, key)), [updateDraft]);

  /** Switch between the site's printings and the player's preferred ones, keeping their edits. */
  const setPrintings = useCallback(async (printings: PrintingPolicy) => {
    if (step.kind !== 'preview' || step.draft.printings === printings) return;
    const current = step.draft;
    const ticket = ++run.current;
    updateDraft((draft) => ({ ...draft, printings }));
    try {
      const lines = [...current.list.commanders, ...current.list.main, ...current.list.side, ...current.list.companion];
      const cards = await resolveCards(lines, cardLookup, { fixes, printings });
      if (ticket === run.current) updateDraft((draft) => ({ ...draft, cards, printings }));
    } catch {
      if (ticket === run.current) updateDraft((draft) => ({ ...draft, printings: current.printings }));
    }
  }, [step, fixes, updateDraft]);

  return {
    text, setText, step, recognized, degraded,
    submit, seed, cancel, backToInput, finishAssisted, updateDraft, pick, skip, setPrintings,
    issues: step.kind === 'preview' ? issuesOf(step.draft) : [],
  };
}

/** "#deck=…" from the bookmarklet, if the current address carries one. */
export function payloadFromLocation(hash: string) {
  return hash.includes('deck=') ? decodePayload(hash) : null;
}
