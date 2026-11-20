import { GripVertical, Plus, X } from 'lucide-react';
import { useEffect, useLayoutEffect, useMemo, useRef, useState, type PointerEvent as ReactPointerEvent } from 'react';
import { useT, type MessageKey } from '../i18n';
import { Button } from '../ui/Button';
import { Dialog } from '../ui/Dialog';
import {
  add, configure, move, moveTo, preset, PRESET_IDS, remove, resize, restore, visible,
  type HomeLayout, type LayoutItem, type WidgetNeed, type WidgetSize,
} from './layoutModel';
import { useHomeLayout } from './layoutStore';
import { WidgetFrame } from './WidgetFrame';
import { widget, WIDGETS } from './widgets';
import { SIZE_CLASS } from './sizeClass';
import styles from './HomeBoard.module.css';

const SIZE_KEY: Record<WidgetSize, MessageKey> = {
  s: 'home.size.s',
  m: 'home.size.m',
  l: 'home.size.l',
  w: 'home.size.w',
};

/**
 * Customize (#432): the board in edit mode. Drag a widget by its handle (or focus the handle and use the arrow keys),
 * pick its size and settings, remove it (with undo), add from the gallery, or start from a preset. Done saves to the
 * account; Escape or Cancel leaves it as it was.
 */
export default function Customize({ has, onDone }: { has: Readonly<Record<WidgetNeed, boolean>>; onDone(): void }) {
  const t = useT();
  const saved = useHomeLayout((state) => state.layout);
  const save = useHomeLayout((state) => state.save);
  const [draft, setDraft] = useState<HomeLayout>(saved);
  const [undo, setUndo] = useState<{ item: LayoutItem; index: number } | null>(null);
  const [gallery, setGallery] = useState(false);
  const [said, setSaid] = useState('');
  const [dragging, setDragging] = useState<string | null>(null);
  const focusKey = useRef<string | null>(null);
  const root = useRef<HTMLElement>(null);
  const items = useMemo(() => visible(draft, WIDGETS, has), [draft, has]);
  const name = (item: LayoutItem) => {
    const def = widget(item.id);
    return def ? t(def.title) : item.id;
  };

  // Escape leaves without saving. It's checked in the capture phase, before a dialog open on top (the gallery) closes
  // itself, so that Escape only closes the dialog
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape' && !event.defaultPrevented && !document.querySelector('[role="dialog"]')) {
        event.preventDefault();
        onDone();
      }
    };
    window.addEventListener('keydown', onKey, true);
    return () => window.removeEventListener('keydown', onKey, true);
  }, [onDone]);

  // a widget moved with the keyboard keeps focus on its handle
  useLayoutEffect(() => {
    const key = focusKey.current;
    if (!key) return;
    focusKey.current = null;
    root.current?.querySelector<HTMLElement>(`[data-handle="${CSS.escape(key)}"]`)?.focus();
  });

  function keyMove(item: LayoutItem, by: number) {
    const next = move(draft, item.key, by);
    if (next === draft) return;
    focusKey.current = item.key;
    setDraft(next);
    const at = next.items.findIndex((entry) => entry.key === item.key);
    setSaid(t('home.board.moved', { name: name(item), place: at + 1, count: next.items.length }));
  }

  function startDrag(event: ReactPointerEvent<HTMLElement>, item: LayoutItem) {
    if (event.button !== 0) return;
    // no text selection while dragging, but the handle still takes focus so the arrow keys work after a click
    event.preventDefault();
    event.currentTarget.focus();
    setDragging(item.key);
    const onMove = (move: PointerEvent) => {
      const over = document.elementFromPoint(move.clientX, move.clientY)?.closest<HTMLElement>('[data-key]');
      const onto = over?.dataset.key;
      if (onto && onto !== item.key) setDraft((current) => moveTo(current, item.key, onto));
    };
    const onUp = () => {
      setDragging(null);
      window.removeEventListener('pointermove', onMove);
      window.removeEventListener('pointerup', onUp);
      window.removeEventListener('pointercancel', onUp);
    };
    window.addEventListener('pointermove', onMove);
    window.addEventListener('pointerup', onUp);
    window.addEventListener('pointercancel', onUp);
  }

  function drop(item: LayoutItem) {
    const result = remove(draft, item.key);
    setDraft(result.layout);
    setUndo(result.removed);
    setSaid(t('home.board.removed', { name: name(item) }));
  }

  async function done() {
    await save(draft);
    onDone();
  }

  const available = WIDGETS.filter((def) => !def.needs || has[def.needs]);

  return (
    <section ref={root} className={[styles.board, styles.editing].join(' ')} aria-label={t('home.board.editing')}>
      <div className={styles.editBar}>
        <div className={styles.presets} role="group" aria-label={t('home.board.presets')}>
          {PRESET_IDS.map((id) => (
            <button
              key={id}
              type="button"
              className={[styles.preset, draft.preset === id ? styles.presetOn : ''].join(' ')}
              aria-pressed={draft.preset === id}
              onClick={() => {
                setDraft(preset(id));
                setUndo(null);
              }}
            >
              {t(`home.preset.${id}`)}
            </button>
          ))}
        </div>
        <span className={styles.editActions}>
          <Button variant="quiet" size="sm" icon={<Plus size={14} />} onClick={() => setGallery(true)}>{t('home.board.add')}</Button>
          <Button variant="quiet" size="sm" onClick={onDone}>{t('home.board.cancel')}</Button>
          <Button variant="decision" size="sm" onClick={() => void done()}>{t('home.board.done')}</Button>
        </span>
      </div>
      <p className={styles.hint}>{t('home.board.hint')}</p>
      {undo && (
        <p className={styles.undo} role="status">
          {t('home.board.removed', { name: name(undo.item) })}
          <button type="button" onClick={() => {
            setDraft(restore(draft, undo));
            setUndo(null);
          }}>{t('home.board.undo')}</button>
        </p>
      )}
      <p className={styles.srOnly} aria-live="polite">{said}</p>
      {items.length === 0 && <p className={styles.empty}>{t('home.board.empty')}</p>}
      <div className={styles.grid}>
        {items.map((item) => {
          const def = widget(item.id);
          return (
            <div key={item.key} data-key={item.key} className={[styles.cell, SIZE_CLASS[item.size], dragging === item.key ? styles.lifted : ''].join(' ')}>
              <div className={styles.tools}>
                <button
                  type="button"
                  className={styles.handle}
                  data-handle={item.key}
                  aria-label={t('home.board.move', { name: name(item) })}
                  aria-describedby="home-board-hint"
                  onPointerDown={(event) => startDrag(event, item)}
                  onKeyDown={(event) => {
                    const by = event.key === 'ArrowLeft' || event.key === 'ArrowUp' ? -1 : event.key === 'ArrowRight' || event.key === 'ArrowDown' ? 1 : 0;
                    if (by) {
                      event.preventDefault();
                      keyMove(item, by);
                    }
                  }}
                >
                  <GripVertical size={16} aria-hidden="true" />
                </button>
                {def && def.sizes.length > 1 && (
                  <select
                    className={styles.select}
                    aria-label={t('home.board.size', { name: name(item) })}
                    value={item.size}
                    onChange={(event) => setDraft(resize(draft, item.key, event.target.value as WidgetSize, WIDGETS))}
                  >
                    {def.sizes.map((size) => <option key={size} value={size}>{t(SIZE_KEY[size])}</option>)}
                  </select>
                )}
                {def?.settings && Object.entries(def.settings).map(([setting, choices]) => choices.length > 0 && (
                  <select
                    key={setting}
                    className={styles.select}
                    aria-label={t('home.board.setting', { name: name(item) })}
                    value={item.settings?.[setting] ?? choices[0]}
                    onChange={(event) => setDraft(configure(draft, item.key, setting, event.target.value))}
                  >
                    {choices.map((choice) => <option key={choice} value={choice}>{t(`home.choice.${choice}` as MessageKey)}</option>)}
                  </select>
                ))}
                <button type="button" className={styles.remove} aria-label={t('home.board.remove', { name: name(item) })} onClick={() => drop(item)}>
                  <X size={14} aria-hidden="true" />
                </button>
              </div>
              <div className={styles.preview} inert>
                <WidgetFrame item={item} editing />
              </div>
            </div>
          );
        })}
      </div>
      <p id="home-board-hint" className={styles.srOnly}>{t('home.board.keys')}</p>
      <Dialog open={gallery} onOpenChange={setGallery} title={t('home.board.gallery')} description={t('home.board.galleryNote', { count: draft.items.length })}>
        <ul className={styles.gallery}>
          {available.map((def) => (
            <li key={def.id}>
              <div>
                <b>{t(def.title)}</b>
                <p>{t(def.blurb)}</p>
                <small>{def.sizes.map((size) => t(SIZE_KEY[size])).join(' · ')}</small>
              </div>
              <Button variant="quiet" size="sm" icon={<Plus size={14} />} onClick={() => {
                setDraft(add(draft, def));
                setGallery(false);
                setSaid(t('home.board.added', { name: t(def.title) }));
              }}>
                {t('home.board.addOne')}
              </Button>
            </li>
          ))}
        </ul>
      </Dialog>
    </section>
  );
}
