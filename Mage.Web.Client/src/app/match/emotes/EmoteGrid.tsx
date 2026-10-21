import * as DropdownMenu from '@radix-ui/react-dropdown-menu';
import { useEffect, useMemo, useState } from 'react';
import { useI18n, useT } from '../../i18n';
import { useCareerUnlocks } from '../../career/careerData';
import { usePaintedArt } from '../../career/careerArt';
import { catalogName, useCosmetics } from '../../ui/cosmeticsCatalog';
import { EMOTE_COOLDOWN_MS, ownedEmoteSets, stickerArt, stickerPhrase } from './emoteSets';
import { useEmoteWheel } from './emoteStore';
import styles from './Emotes.module.css';

/** Whether the wheel is waiting out the server's pause between emotes; true until it's over. */
function useCooling(): boolean {
  const sentAt = useEmoteWheel((state) => state.sentAt);
  // the send whose wait is over
  const [over, setOver] = useState(0);
  useEffect(() => {
    if (!sentAt) return;
    const timer = setTimeout(() => setOver(sentAt), Math.max(0, sentAt + EMOTE_COOLDOWN_MS - Date.now()));
    return () => clearTimeout(timer);
  }, [sentAt]);
  return sentAt > 0 && over !== sentAt;
}

/** The wheel's contents: each set you can send from, its phrases as words and its stickers as pictures. */
export default function EmoteGrid({ gameId }: { gameId: string }) {
  const t = useT();
  const locale = useI18n((state) => state.locale);
  const items = useCosmetics((state) => state.items);
  const unlocks = useCareerUnlocks();
  const paintedArt = usePaintedArt();
  const sets = useMemo(() => ownedEmoteSets(items, unlocks), [items, unlocks]);
  const send = useEmoteWheel((state) => state.send);
  const cooling = useCooling();
  return (
    <>
      {cooling && <p className={styles.wait} role="status">{t('match.emote.wait')}</p>}
      {sets.map((set) => {
        const pictures = !set.free && paintedArt;
        return (
          <DropdownMenu.Group key={set.id} className={styles.set}>
            <DropdownMenu.Label className={styles.setName}>{catalogName(set.item, locale)}</DropdownMenu.Label>
            <div className={pictures ? styles.stickers : styles.phrases}>
              {set.stickers.map((sticker) => {
                const phrase = stickerPhrase(sticker, locale);
                return (
                  <DropdownMenu.Item
                    key={sticker.id}
                    className={pictures ? styles.stickerItem : styles.phraseItem}
                    disabled={cooling}
                    onSelect={() => void send(gameId, sticker.id!)}
                    textValue={phrase}
                    title={pictures ? phrase : undefined}
                  >
                    {pictures ? <StickerPicture id={sticker.id!} phrase={phrase} /> : phrase}
                  </DropdownMenu.Item>
                );
              })}
            </div>
          </DropdownMenu.Group>
        );
      })}
    </>
  );
}

/** A sticker's picture, or its words while the painted art isn't there. */
function StickerPicture({ id, phrase }: { id: string; phrase: string }) {
  const [failed, setFailed] = useState(false);
  if (failed) return <span className={styles.stickerWords}>{phrase}</span>;
  return <img src={stickerArt(id)} alt={phrase} draggable={false} loading="lazy" onError={() => setFailed(true)} />;
}
