/**
 * The thank-you set and which of it a supporter wears, without imports, so the app shell can check a look (the ring by
 * the account menu) without loading the rest of supportModel.
 */

/** The thank-you set (Mage.Server support/SupportPerks THANK_YOU): one item per kind, all cosmetic. */
export const THANK_YOU = { sleeve: 'supporter', playmat: 'patron', frame: 'supporter-ring', title: 'patron', flair: 'heart', pet: 'party-corgi' } as const;

/** Which of the thank-you set a supporter shows (Settings, Account); all of it unless they turn a piece off. */
export interface Wear {
  frame: boolean;
  flair: boolean;
  title: boolean;
  /** bring the pet to the table */
  pet: boolean;
}

export const DEFAULT_WEAR: Wear = { frame: true, flair: true, title: true, pet: true };
