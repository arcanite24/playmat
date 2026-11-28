import type { DeckCardInfo as WireCard, ImportedDeck } from '../../protocol/generated/views';
import type { DeckCardInfo } from '../decks/types';
import { RPC_ERROR, RpcError } from '../rpc/RpcClient';
import type { DeckReading } from './draft';
import { siteById, type DeckSiteId } from './sites';

export type ImportFailureReason =
  | 'private' | 'not_found' | 'blocked' | 'rate_limited' | 'site_changed'
  | 'unsupported' | 'too_large' | 'timeout' | 'unavailable' | 'offline';

/** Why a link import failed, in words, and whether pasting the list is the way forward. */
export interface ImportFailure {
  reason: ImportFailureReason;
  message: string;
  /** offer the site's copy-and-paste flow instead */
  assisted: boolean;
}

const REASONS: ImportFailureReason[] = ['private', 'not_found', 'blocked', 'rate_limited', 'site_changed', 'unsupported', 'too_large', 'timeout', 'unavailable'];

function cards(list: WireCard[] | undefined): DeckCardInfo[] {
  return (list ?? []).map((card) => ({
    amount: card.amount ?? 1,
    cardName: card.cardName ?? '',
    setCode: card.setCode || null,
    cardNumber: card.cardNumber || null,
  })).filter((card) => card.cardName);
}

/** The server's reading of a deck link, in the shape a pasted list reads into. */
export function readingFromServer(deck: ImportedDeck, site: DeckSiteId, fallbackUrl: string): DeckReading {
  return {
    list: {
      main: cards(deck.main),
      side: cards(deck.side),
      commanders: cards(deck.commanders),
      companion: cards(deck.companion),
      maybe: [],
      skipped: [],
    },
    maybeCount: deck.maybeboardCount ?? 0,
    name: deck.name,
    author: deck.author,
    format: deck.format,
    description: deck.description,
    cover: deck.cover?.setCode && deck.cover.cardNumber
      ? { setCode: deck.cover.setCode, cardNumber: deck.cover.cardNumber, name: deck.cover.cardName }
      : undefined,
    source: {
      site,
      url: deck.url || fallbackUrl,
      remoteId: deck.remoteId ?? '',
      remoteUpdatedAt: deck.remoteUpdatedAt ?? undefined,
    },
  };
}

/** Turns a failed deckImportFromUrl call into what the import sheet tells the player. */
export function describeFailure(error: unknown, site: DeckSiteId): ImportFailure {
  const name = siteById(site)?.name ?? 'The site';
  let reason: ImportFailureReason = 'unavailable';
  if (error instanceof RpcError) {
    const data = error.data as { reason?: string } | undefined;
    if (error.code === RPC_ERROR.DECK_IMPORT_FAILED && data?.reason && (REASONS as string[]).includes(data.reason)) {
      reason = data.reason as ImportFailureReason;
    } else if (error.code === RPC_ERROR.RATE_LIMITED) {
      reason = 'rate_limited';
    } else if (error.code === RPC_ERROR.METHOD_NOT_FOUND) {
      // a server from before link imports
      reason = 'unsupported';
    }
  } else {
    reason = 'offline';
  }

  switch (reason) {
    case 'private':
      return { reason, assisted: true, message: `This deck is private. Make it public or unlisted on ${name}, or paste its list.` };
    case 'not_found':
      return { reason, assisted: false, message: `${name} has no deck at this link. Check the link, or paste the list.` };
    case 'blocked':
    case 'site_changed':
      return { reason, assisted: true, message: `${name} changed how its decks load. Paste the list for now.` };
    case 'rate_limited':
      return { reason, assisted: false, message: 'Too many imports at once. Try again in a minute.' };
    case 'too_large':
      return { reason, assisted: true, message: `That's more than a deck's worth of cards. Paste the list instead.` };
    case 'timeout':
      return { reason, assisted: true, message: `${name} took too long to answer. Try again, or paste the list.` };
    case 'unsupported':
      return { reason, assisted: true, message: `This server can't read ${name} links yet. Paste the list instead.` };
    case 'offline':
      return { reason, assisted: true, message: 'Not connected to the server. Paste the list, or try again once you’re back online.' };
    default:
      return { reason: 'unavailable', assisted: true, message: `${name} isn't answering right now. Try again, or paste the list.` };
  }
}
