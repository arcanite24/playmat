import { ArrowLeft, ArrowRight, LogOut, ZoomIn } from 'lucide-react';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { createPortal } from 'react-dom';
import { useNavigate, useParams } from 'react-router-dom';
import type { CardView } from '../../protocol/generated/views';
import { isEditableEventTarget } from '../ui/keys';
import { api } from '../connection';
import { CardInspect } from '../decks/CardInspect';
import { useCardInfo } from '../decks/cardInfo';
import { registerMessages, useT } from '../i18n';
import eventMessages from '../i18n/en/events';
import { useLongPress } from '../match/useLongPress';
import { useEvents } from '../stores/events';
import { useSession } from '../stores/session';
import { Button } from '../ui/Button';
import { CardFace } from '../ui/CardFace';
import { Dialog } from '../ui/Dialog';
import { curveColumns, faceOf, poolEntry, type PoolCard } from './pool';
import styles from './Draft.module.css';

registerMessages(eventMessages);

const NONE: PoolCard[] = [];

/** A booster draft: the pack in front of you, your picks below, the table around. */
export function DraftScreen() {
  const { draftId = '' } = useParams();
  const navigate = useNavigate();
  const t = useT();
  const draft = useEvents((state) => state.drafts[draftId]);
  const pick = useEvents((state) => state.pick);
  const mark = useEvents((state) => state.mark);
  const userName = useSession((state) => state.userName);
  const [selected, setSelected] = useState<string | null>(null);
  const [leaving, setLeaving] = useState(false);
  const [preview, setPreview] = useState<{ card: CardView; anchor: DOMRect } | null>(null);
  const [inspected, setInspected] = useState<CardView | null>(null);
  const [screenEl, setScreenEl] = useState<HTMLDivElement | null>(null);

  const booster = (draft?.booster ?? NONE) as PoolCard[];
  const picks = (draft?.picks ?? NONE) as PoolCard[];
  const entries = useMemo(() => [...booster, ...picks].map(poolEntry), [booster, picks]);
  const info = useCardInfo(entries);
  const columns = useMemo(() => curveColumns(picks, info), [picks, info]);
  const cardsById = useMemo(() => {
    const map = new Map<string, PoolCard>();
    for (const card of booster) if (card.id) map.set(card.id, card);
    for (const card of picks) if (card.id) map.set(card.id, card);
    return map;
  }, [booster, picks]);

  const view = draft?.view;
  const packNumber = view?.boosterNum ?? 1;
  const pickNumber = view?.cardNum ?? picks.length + 1;
  const passLeft = packNumber % 2 === 1;
  const setName = view?.setNames?.[Math.max(0, packNumber - 1)] ?? view?.setCodes?.[Math.max(0, packNumber - 1)] ?? '';

  const openInspect = useCallback((card: CardView) => {
    setPreview(null);
    setInspected(card);
  }, []);

  const showPreview = useCallback((card: CardView, event: React.PointerEvent<HTMLElement>) => {
    if (event.pointerType === 'touch' || event.pointerType === 'pen') return;
    setPreview({ card, anchor: event.currentTarget.getBoundingClientRect() });
  }, []);

  // touch has no hover and no right-click: a long press on any card opens it up close
  useLongPress(screenEl, {
    onLongPress: (target) => {
      if (inspected) return false;
      const el = target.closest<HTMLElement>('[data-draft-card-id]');
      const id = el?.dataset.draftCardId;
      const raw = id ? cardsById.get(id) : undefined;
      if (!raw) return false;
      openInspect(faceOf(raw, info));
      return true;
    },
  });

  // a new pack clears the choice and hover preview
  const boosterKey = booster.map((card) => card.id).join(',');
  const [shownKey, setShownKey] = useState(boosterKey);
  if (shownKey !== boosterKey) {
    setShownKey(boosterKey);
    setSelected(null);
    setPreview(null);
  }

  const confirm = (cardId: string | null) => {
    if (cardId && draft?.picking) void pick(draftId, cardId);
  };

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (isEditableEventTarget(event.target) || inspected || leaving) return;
      if ((event.code === 'Space' || event.key === 'Enter') && selected) {
        event.preventDefault();
        confirm(selected);
      } else if ((event.key === 'i' || event.key === 'I') && selected) {
        const raw = cardsById.get(selected);
        if (raw) {
          event.preventDefault();
          openInspect(faceOf(raw, info));
        }
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  });

  if (!draft) {
    return (
      <div className={styles.screen}>
        <div className={styles.center}>
          <h1 className={styles.title}>No draft here</h1>
          <p className={styles.sub}>This draft isn't open in this tab.</p>
          <Button variant="print" onClick={() => navigate('/events')}>Back to events</Button>
        </div>
      </div>
    );
  }

  return (
    <div ref={setScreenEl} className={styles.screen}>
      <header className={styles.top}>
        <div className={styles.where}>
          <span className={styles.set}>{setName}</span>
          <h1 className={styles.title}>Pack {packNumber} <span>· Pick {pickNumber}</span></h1>
        </div>
        <Seats players={view?.players ?? []} me={userName} passLeft={passLeft} />
        <div className={styles.topRight}>
          {draft.picking && draft.deadline && <PickClock deadline={draft.deadline} />}
          <Button variant="quiet" size="sm" icon={<LogOut size={16} />} onClick={() => setLeaving(true)}>Leave</Button>
        </div>
      </header>

      <main className={styles.pack} aria-label="Booster">
        {draft.over ? (
          <div className={styles.center}>
            <h2 className={styles.title}>Draft complete</h2>
            <p className={styles.sub}>{picks.length} cards drafted. Deck building opens in a moment.</p>
          </div>
        ) : !draft.picking ? (
          <div className={styles.center}>
            <h2 className={styles.waitTitle}>Waiting for the next pack</h2>
            <p className={styles.sub}>The others are still picking.</p>
          </div>
        ) : (
          <div className={styles.booster} role="listbox" aria-label={`Pick a card, ${booster.length} left in the pack`}>
            {booster.map((card, index) => {
              const id = card.id!;
              const isSelected = selected === id;
              const face = faceOf(card, info);
              const inspectLabel = card.name ? t('events.draft.inspect', { name: card.name }) : t('events.draft.inspectFallback');
              return (
                <div
                  key={id}
                  className={styles.cardWrap}
                  data-draft-card-id={id}
                  onPointerEnter={(event) => showPreview(face, event)}
                  onPointerLeave={() => setPreview(null)}
                >
                  <button
                    type="button"
                    role="option"
                    aria-selected={isSelected}
                    aria-label={card.name}
                    className={[styles.card, isSelected ? styles.cardSelected : ''].join(' ')}
                    style={{ animationDelay: `${index * 25}ms` }}
                    onClick={() => {
                      setSelected(id);
                      mark(draftId, id);
                    }}
                    onDoubleClick={() => confirm(id)}
                    onContextMenu={(event) => {
                      event.preventDefault();
                      openInspect(face);
                    }}
                  >
                    <CardFace card={face} size="normal" />
                  </button>
                  <button
                    type="button"
                    className={styles.inspectButton}
                    aria-label={inspectLabel}
                    title={inspectLabel}
                    onClick={(event) => {
                      event.stopPropagation();
                      openInspect(face);
                    }}
                  >
                    <ZoomIn size={16} aria-hidden="true" />
                  </button>
                </div>
              );
            })}
          </div>
        )}
        {draft.picking && (
          <div className={styles.action}>
            <p className={styles.hint}>{selected ? t('events.draft.hintSelected') : t('events.draft.hintChoose')}</p>
            <Button variant="decision" size="xl" disabled={!selected} onClick={() => confirm(selected)}>Pick</Button>
          </div>
        )}
      </main>

      <section className={styles.picks} aria-label={`Your picks, ${picks.length} cards`}>
        <h2 className={styles.picksLabel}>Your picks <span>{picks.length}</span></h2>
        <div className={styles.columns}>
          {columns.map((column) => (
            <div key={column.label} className={styles.column}>
              <span className={styles.columnLabel}>{column.label}</span>
              <div className={styles.columnCards} style={{ height: 150 + (column.cards.length - 1) * 26 }}>
                {column.cards.map((card, index) => {
                  const face = faceOf(card, info);
                  const inspectLabel = card.name ? t('events.draft.inspect', { name: card.name }) : t('events.draft.inspectFallback');
                  return (
                    <button
                      key={card.id}
                      type="button"
                      className={styles.picked}
                      data-draft-card-id={card.id}
                      style={{ top: index * 26, zIndex: index }}
                      aria-label={inspectLabel}
                      onClick={() => openInspect(face)}
                      onContextMenu={(event) => {
                        event.preventDefault();
                        openInspect(face);
                      }}
                      onPointerEnter={(event) => showPreview(face, event)}
                      onPointerLeave={() => setPreview(null)}
                    >
                      <CardFace card={face} size="small" />
                    </button>
                  );
                })}
              </div>
            </div>
          ))}
        </div>
      </section>

      {preview && !inspected && <DraftPreview card={preview.card} anchor={preview.anchor} />}
      <CardInspect card={inspected} onClose={() => setInspected(null)} />

      <Dialog
        open={leaving}
        onOpenChange={setLeaving}
        title="Leave the draft?"
        description="You can't come back to this draft, and you'll leave the event."
        width="sm"
        footer={(
          <>
            <Button variant="quiet" onClick={() => setLeaving(false)}>Keep drafting</Button>
            <Button
              variant="danger"
              onClick={() => {
                api.draftQuit(draftId).catch(() => undefined);
                navigate('/events');
              }}
            >
              Leave
            </Button>
          </>
        )}
      >
        {null}
      </Dialog>
    </div>
  );
}

/** Floating desktop hover preview beside a hovered booster or picked card (#467). */
function DraftPreview({ card, anchor }: { card: CardView; anchor: DOMRect }) {
  const width = 300;
  const height = width * (88 / 63);
  const gap = 16;
  const back = card.transformable ? card.secondCardFace : undefined;
  const span = back ? width * 1.62 + 12 : width;
  const fitsRight = anchor.right + gap + span <= window.innerWidth - gap;
  const left = fitsRight ? anchor.right + gap : Math.max(gap, anchor.left - gap - span);
  const top = Math.max(12, Math.min(window.innerHeight - height - 12, anchor.top + anchor.height / 2 - height / 2));
  return createPortal(
    <div className={styles.preview} style={{ left, top, width: span }} aria-hidden="true">
      <div className={styles.previewFaces}>
        <CardFace card={card} size="large" style={{ width }} />
        {back && <CardFace card={back} face="back" size="normal" style={{ width: width * 0.62 }} />}
      </div>
    </div>,
    document.body,
  );
}

/** Who sits where, and which way the packs go this round. */
function Seats({ players, me, passLeft }: { players: string[]; me: string; passLeft: boolean }) {
  if (players.length === 0) return null;
  return (
    <ol className={styles.seats} aria-label={`Players, packs pass ${passLeft ? 'left' : 'right'}`}>
      <li className={styles.direction} aria-hidden="true">{passLeft ? <ArrowLeft size={16} /> : <ArrowRight size={16} />}</li>
      {players.map((name, index) => (
        <li key={`${name}-${index}`} className={[styles.seat, name === me ? styles.seatMe : ''].join(' ')} title={name}>
          {name.slice(0, 1).toUpperCase()}
        </li>
      ))}
    </ol>
  );
}

/** Time left for this pick, as a ring that empties. */
function PickClock({ deadline }: { deadline: number }) {
  const [now, setNow] = useState(() => Date.now());
  const [total] = useState(() => Math.max(1, deadline - Date.now()));
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 250);
    return () => clearInterval(timer);
  }, []);
  const left = Math.max(0, deadline - now);
  const seconds = Math.ceil(left / 1000);
  return (
    <div className={[styles.clock, seconds <= 10 ? styles.clockLow : ''].join(' ')} role="timer" aria-label={`${seconds} seconds left to pick`}>
      <svg viewBox="0 0 44 44" aria-hidden="true">
        <circle cx="22" cy="22" r="19" className={styles.clockTrack} />
        <circle cx="22" cy="22" r="19" className={styles.clockFill} pathLength={100} strokeDasharray={`${(left / total) * 100} 100`} />
      </svg>
      <span>{seconds}</span>
    </div>
  );
}
