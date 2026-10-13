/** A deck as the client keeps and edits it (the wire form is DeckCardLists in protocol/generated/views). */

export interface DeckCardInfo {
  cardName: string;
  setCode?: string | null;
  cardNumber?: string | null;
  amount: number;
}

export interface DeckCardLayout {
  // Layout information for deck editor
  [key: string]: unknown;
}

export interface DeckCoverCard {
  setCode: string;
  cardNumber: string;
  name?: string;
}

export interface DeckImportSource {
  /** a site id from core/deckImport/sites */
  site: string;
  url: string;
  remoteId: string;
  importedAt: number;
  remoteUpdatedAt?: number;
  /** the deck's name when it was imported; an update renames the deck only if the player kept that name */
  importedName?: string;
}

/** A number of copies of a card, by name, in a sideboard plan. */
export interface PlanCard {
  name: string;
  count: number;
}

/** What to bring in and take out against one kind of deck (S018). */
export interface SideboardPlan {
  id: string;
  /** "Aggro", "Control", or the player's own label */
  archetype: string;
  /** applied on its own when sideboarding in a best-of-three against the computer */
  default?: boolean;
  in: PlanCard[];
  out: PlanCard[];
  note?: string;
}

export interface DeckCardLists {
  id?: string;
  name?: string;
  description?: string;
  author?: string;
  format?: string; // e.g. "Constructed - Standard", syncs with deckType
  cards: DeckCardInfo[];
  sideboard: DeckCardInfo[];
  cardLayout?: DeckCardLayout;
  sideboardLayout?: DeckCardLayout;
  coverCard?: DeckCoverCard;
  /** the deck website this deck was imported from, so it can be updated later */
  source?: DeckImportSource;
  colors?: {
    white: boolean;
    blue: boolean;
    black: boolean;
    red: boolean;
    green: boolean;
  };
  createdAt?: number;
  updatedAt?: number;
  /** what to swap against each kind of deck between games */
  sideboardPlans?: SideboardPlan[];
}
