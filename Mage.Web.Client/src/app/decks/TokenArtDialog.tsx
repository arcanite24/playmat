import type { DeckCardLists } from '../../core/decks/types';
import { registerMessages, useT } from '../i18n';
import messages from '../i18n/en/settings';
import { useTokenArt } from '../stores/tokenArt';
import { Dialog } from '../ui/Dialog';
import { TokenArtPicker } from '../ui/TokenArtPicker';

registerMessages(messages);

/**
 * Token art for one deck (ui/tokenArt.ts): its own picture for some tokens, over the account's choices. Kept with
 * the deck, so it syncs with it, and sent with the look while the deck is played.
 */
export function TokenArtDialog({ open, onOpenChange, deck, onChange }: {
  open: boolean;
  onOpenChange(open: boolean): void;
  deck: DeckCardLists;
  onChange(update: (current: DeckCardLists) => DeckCardLists): void;
}) {
  const t = useT();
  const account = useTokenArt((state) => state.choices);
  const choose = (name: string, picture: string | null) => onChange((current) => {
    const tokenArt = { ...(current.tokenArt ?? {}) };
    if (picture) tokenArt[name] = picture;
    else delete tokenArt[name];
    return { ...current, tokenArt: Object.keys(tokenArt).length > 0 ? tokenArt : undefined };
  });
  return (
    <Dialog open={open} onOpenChange={onOpenChange} title={t('tokenArt.deckTitle')} description={t('tokenArt.deckIntro')} width="lg">
      <TokenArtPicker choices={deck.tokenArt ?? {}} onChoose={choose} inherited={account} />
    </Dialog>
  );
}
