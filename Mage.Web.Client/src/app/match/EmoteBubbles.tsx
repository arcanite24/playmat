import { useLayoutEffect, useState } from 'react';
import type { GameView } from '../../protocol/generated/views';
import { useI18n, useT } from '../i18n';
import { usePaintedArt } from '../career/careerArt';
import { useCosmetics } from '../ui/cosmeticsCatalog';
import { useEmotes, type EmoteBubble } from './emotes';
import { findSticker, stickerArt, stickerPhrase } from './emotes/emoteSets';
// the server's gameEmote callback, turned into bubbles here
import './emotes/emoteStore';
import { toStagePoint, useStage } from './stageContext';
import styles from './EmoteBubbles.module.css';

/** Emotes as short speech bubbles beside the sender's seat. */
export function EmoteBubbles({ view, gameId }: { view: GameView | null; gameId?: string }) {
  const bubbles = useEmotes((state) => state.bubbles);
  const players = view?.players;
  return (
    <>
      {bubbles.map((bubble) => {
        // a quick emote belongs to its game: another open game's stay there
        if (bubble.gameId && gameId && bubble.gameId !== gameId) return null;
        const playerId = players?.find((player) => player.name === bubble.from)?.playerId;
        return playerId ? <Bubble key={bubble.key} bubble={bubble} playerId={playerId} /> : null;
      })}
    </>
  );
}

function Bubble({ bubble, playerId }: { bubble: EmoteBubble; playerId: string }) {
  const stage = useStage();
  const [at, setAt] = useState<{ left: number; top: number } | null>(null);
  // the seat is measured once the bubble mounts: seats don't move while a bubble lasts
  useLayoutEffect(() => {
    const plate = stage.element?.querySelector(`[data-object-id="${CSS.escape(playerId)}"]`);
    if (!plate) return;
    const box = plate.getBoundingClientRect();
    const corner = toStagePoint(stage, box.right, box.top);
    // eslint-disable-next-line react-hooks/set-state-in-effect -- measuring the DOM must happen after layout
    setAt({ left: corner.x + 14, top: corner.y + 6 });
  }, [stage, playerId]);
  if (!at) return null;
  if (bubble.sticker) {
    return (
      <div className={[styles.bubble, styles.quick].join(' ')} style={at} role="status">
        <Sticker id={bubble.sticker} from={bubble.from} />
      </div>
    );
  }
  return (
    <div className={styles.bubble} style={at} role="status" aria-label={`${bubble.from}: ${bubble.text}`}>
      {bubble.text}
    </div>
  );
}

/** A quick emote: an earned set's painted sticker, or its words (free phrases, painted art off, or the art not there yet). */
function Sticker({ id, from }: { id: string; from: string }) {
  const t = useT();
  const locale = useI18n((state) => state.locale);
  const items = useCosmetics((state) => state.items);
  const paintedArt = usePaintedArt();
  const [failed, setFailed] = useState(false);
  const found = findSticker(id, items);
  const phrase = stickerPhrase(found?.sticker, locale) || id;
  const label = t('match.emote.bubble', { name: from, phrase });
  if (found && !found.free && paintedArt && !failed) {
    return <img className={styles.sticker} src={stickerArt(id)} alt={label} draggable={false} onError={() => setFailed(true)} />;
  }
  return <span aria-label={label}>{phrase}</span>;
}
