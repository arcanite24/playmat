import { ClipboardPaste, Cloud, CloudOff, Copy, Download, PencilLine, PencilRuler, RefreshCw, Trash2 } from 'lucide-react';
import { lazy, Suspense, useEffect, useState, type FormEvent } from 'react';
import { useNavigate } from 'react-router-dom';
import { siteById } from '../../core/deckImport/sites';
import { ExportMenu } from '../decks/ExportMenu';
import { SourceLine } from '../decks/import/SourceLine';
import { UpdateDialog } from '../decks/import/UpdateDialog';
import { useImportDrop, useImportPaste } from '../decks/import/useImportShortcuts';
import { useDegradedSites } from '../decks/import/useImportFlow';
import { PASTE_KEYS } from '../decks/import/text';
import { deckStorage } from '../../core/decks/DeckStorageService';
import { applyUpdate } from '../../core/deckImport/diff';
import { t as translate, useT } from '../i18n';
import { RichText } from '../i18n/RichText';
import { SLEEVE_COLORS, chosenSleeve, sleeveFor, useDecks, type RosterDeck } from '../stores/decks';
import { useDeckRoster } from '../stores/useDeckRoster';
import { useDeckSyncStatus } from '../stores/deckSyncStatus';
import { useSettings } from '../stores/settings';
import { useServerInfo } from '../queries';
import { openImport, useImportSheet } from '../stores/importSheet';
import { Button } from '../ui/Button';
import { DeckBox } from '../ui/DeckBox';
import { CardFace } from '../ui/CardFace';
import { Dialog } from '../ui/Dialog';
import { Field } from '../ui/Field';
import { notify } from '../stores/toasts';
import { Zone } from '../ui/Zone';
import styles from './DecksScreen.module.css';

// sleeves unlocked in Career, for players who opted in
const CareerSleeves = lazy(() => import('../career/CareerScreen').then((m) => ({ default: m.CareerSleeves })));
// the supporter sleeve, on servers with accounts: owned, or a quiet teaser where the server takes support
const SupporterSleeve = lazy(() => import('../support/SupporterPicks').then((m) => ({ default: m.SupporterSleeve })));
const SupporterSleeveTeaser = lazy(() => import('../support/SupporterPicks').then((m) => ({ default: m.SupporterSleeveTeaser })));

// sharing a deck as a ghost, and the ghosts you share (S047); they bring their messages with them
const GhostActions = lazy(() => import('../solo/Ghosts').then((m) => ({ default: m.GhostActions })));

/** Your deck shelf: import, sleeve, update and remove decks. */
export function DecksScreen() {
  const decks = useDecks();
  const t = useT();
  const roster = useDeckRoster();
  const [removing, setRemoving] = useState<RosterDeck | null>(null);
  const [updating, setUpdating] = useState<string | null>(null);
  const [renaming, setRenaming] = useState<RosterDeck | null>(null);
  // a sleeve up close (right-click its swatch)
  const [sleevePreview, setSleevePreview] = useState<{ color: string; name: string } | null>(null);
  const selected = roster.find((deck) => deck.id === decks.selectedId) ?? null;
  const arrivedId = useImportSheet((state) => state.arrivedId);
  const career = useSettings((state) => state.settings.careerOptIn);
  const accounts = useServerInfo().data?.accounts === true;
  const navigate = useNavigate();
  const degraded = useDegradedSites();
  const { dragging, dropProps } = useImportDrop();
  useImportPaste();

  useEffect(() => {
    if (!decks.loaded) void decks.refresh();
  }, [decks]);

  const source = selected?.source;
  const sourceSite = siteById(source?.site);
  const updatable = !!sourceSite && sourceSite.tier === 'direct' && !degraded.has(sourceSite.id);

  async function duplicate(deck: RosterDeck) {
    try {
      await useDecks.getState().duplicate(deck.id);
    } catch (error) {
      notify(translate('decks.duplicateFailed'), error instanceof Error ? error.message : String(error), 'error');
    }
  }

  /** A site that blocks us: paste a fresh copy of the list, which replaces this deck's cards. */
  function pasteNewList(deck: RosterDeck) {
    if (!deck.source) return;
    openImport({ kind: 'text', text: deck.source.url }, {
      kind: 'into',
      mode: 'replace',
      deckName: deck.name,
      apply: (fresh) => {
        void (async () => {
          const saved = await deckStorage.loadDeck(deck.id);
          if (!saved) return;
          await useDecks.getState().saveUpdated({ ...applyUpdate(saved, fresh), id: deck.id });
        })();
      },
    });
  }

  return (
    <div className={styles.page}>
      <Zone
        label={t('decks.yourDecks')}
        className={[styles.shelf, dragging ? styles.dropping : ''].join(' ')}
        {...dropProps}
        aside={
          <>
            <SyncNote />
            <Button size="sm" icon={<Download size={16} />} onClick={() => openImport()}>{t('decks.import')}</Button>
            <Button size="sm" icon={<PencilRuler size={16} />} onClick={() => navigate('/decks/new')}>{t('decks.build')}</Button>
          </>
        }
      >
        {dragging && (
          <div className={styles.dropHint} aria-hidden="true">
            <ClipboardPaste size={22} />
            <span>{t('decks.dropHint')}</span>
          </div>
        )}
        <div className={styles.grid} role="list">
          {decks.loaded && decks.saved.length === 0 && (
            <p role="listitem" className={styles.emptyShelf}>
              <RichText text={t('decks.empty')} parts={{ keys: <kbd>{PASTE_KEYS}</kbd> }} />
            </p>
          )}
          {roster.map((deck) => (
            <div key={deck.id} role="listitem" className={[styles.cell, deck.id === arrivedId ? styles.arrived : ''].join(' ')}>
              <div onDoubleClick={() => navigate(`/decks/${encodeURIComponent(deck.id)}`)}>
                <DeckBox deck={deck} sleeve={sleeveFor(decks.sleeves, deck)} selected={deck.id === decks.selectedId} onSelect={() => decks.select(deck.id)} />
              </div>
            </div>
          ))}
        </div>
      </Zone>

      <Zone tone="filled" className={styles.detail}>
        {selected ? (
          <div className={styles.detailBody}>
            <h2 className={styles.detailName}>{selected.name}</h2>
            <p className={styles.detailNote}>{selected.note}</p>
            {source && sourceSite && (
              <div className={styles.source}>
                <SourceLine source={source} />
                {updatable ? (
                  <Button icon={<RefreshCw size={16} />} onClick={() => setUpdating(selected.id)}>{t('decks.updateFrom', { site: sourceSite.name })}</Button>
                ) : (
                  <Button icon={<ClipboardPaste size={16} />} onClick={() => pasteNewList(selected)}>{t('decks.pasteNewList')}</Button>
                )}
              </div>
            )}
            <div>
              <h3 className={styles.subLabel}>{t('decks.sleeves')}</h3>
              <div className={styles.sleeveRow}>
                <div className={styles.sleeves} role="radiogroup" aria-label={t('decks.sleeveColor')}>
                  {SLEEVE_COLORS.map((color) => (
                    <button
                      key={color}
                      type="button"
                      role="radio"
                      aria-checked={chosenSleeve(decks.sleeves, selected) === color}
                      aria-label={t('decks.sleeve', { color })}
                      className={styles.swatch}
                      style={{ background: color }}
                      onClick={() => decks.setSleeve(selected.id, color)}
                      onContextMenu={(event) => {
                        event.preventDefault();
                        setSleevePreview({ color, name: t('decks.sleeve', { color }) });
                      }}
                    />
                  ))}
                  {career && (
                    <Suspense fallback={null}>
                      <CareerSleeves current={chosenSleeve(decks.sleeves, selected)} swatchClass={styles.swatch} onPick={(color) => decks.setSleeve(selected.id, color)} onPreview={(color, name) => setSleevePreview({ color, name })} />
                    </Suspense>
                  )}
                  {accounts && (
                    <Suspense fallback={null}>
                      <SupporterSleeve current={chosenSleeve(decks.sleeves, selected)} swatchClass={styles.swatch} onPick={(color) => decks.setSleeve(selected.id, color)} onPreview={(color, name) => setSleevePreview({ color, name })} />
                    </Suspense>
                  )}
                </div>
                {accounts && <Suspense fallback={null}><SupporterSleeveTeaser swatchClass={styles.swatch} /></Suspense>}
              </div>
            </div>
            <Button variant="print" icon={<PencilRuler size={16} />} onClick={() => navigate(`/decks/${encodeURIComponent(selected.id)}`)}>
              {selected.starter ? t('decks.copyAndEdit') : t('decks.edit')}
            </Button>
            <div className={styles.actions}>
              <ExportMenu getDeck={() => useDecks.getState().loadList(selected.id)} />
              <Button variant="print" size="sm" icon={<Copy size={16} />} onClick={() => void duplicate(selected)}>{t('decks.duplicate')}</Button>
              {!selected.starter && (
                <Button variant="print" size="sm" icon={<PencilLine size={16} />} onClick={() => setRenaming(selected)}>{t('decks.rename')}</Button>
              )}
              <Suspense fallback={null}>
                <GhostActions deckId={selected.id} deckName={selected.name} />
              </Suspense>
            </div>
            {!selected.starter && (
              <Button variant="danger" size="sm" className={styles.remove} icon={<Trash2 size={16} />} onClick={() => setRemoving(selected)}>{t('decks.remove')}</Button>
            )}
          </div>
        ) : (
          <p className={styles.detailNote}>{t('decks.pick')}</p>
        )}
      </Zone>

      <UpdateDialog deckId={updating} onClose={() => setUpdating(null)} />

      <RenameDialog deck={renaming} onClose={() => setRenaming(null)} />

      <Dialog
        open={!!sleevePreview}
        onOpenChange={(open) => !open && setSleevePreview(null)}
        title={sleevePreview?.name ?? ''}
        width="sm"
        footer={<Button variant="quiet" onClick={() => setSleevePreview(null)}>{t('ui.close')}</Button>}
      >
        {sleevePreview && (
          <div className={styles.sleevePreview}>
            <CardFace card={{ name: '' }} hidden sleeve={sleevePreview.color} size="large" />
          </div>
        )}
      </Dialog>

      <Dialog
        open={!!removing}
        onOpenChange={(open) => !open && setRemoving(null)}
        title={t('decks.removeTitle')}
        width="sm"
        description={removing ? t('decks.removeDescription', { name: removing.name }) : undefined}
        footer={
          <>
            <Button variant="quiet" onClick={() => setRemoving(null)}>{t('decks.keep')}</Button>
            <Button
              variant="danger"
              onClick={async () => {
                if (removing) await decks.remove(removing.id);
                setRemoving(null);
              }}
            >
              {t('decks.removeConfirm')}
            </Button>
          </>
        }
      >
        <span />
      </Dialog>
    </div>
  );
}

/** Rename a saved deck in place (same id, sleeve and history). */
function RenameDialog({ deck, onClose }: { deck: RosterDeck | null; onClose(): void }) {
  const t = useT();
  return (
    <Dialog open={!!deck} onOpenChange={(open) => !open && onClose()} title={t('decks.renameTitle')} width="sm">
      {deck && <RenameForm key={deck.id} deck={deck} onClose={onClose} />}
    </Dialog>
  );
}

function RenameForm({ deck, onClose }: { deck: RosterDeck; onClose(): void }) {
  const t = useT();
  const [name, setName] = useState(deck.name);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (!name.trim()) {
      setError(t('decks.nameRequired'));
      return;
    }
    setBusy(true);
    try {
      await useDecks.getState().rename(deck.id, name);
      onClose();
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
      setBusy(false);
    }
  }

  return (
    <form className={styles.renameForm} onSubmit={(event) => void submit(event)}>
      <Field
        label={t('decks.name')}
        value={name}
        maxLength={60}
        autoFocus
        onFocus={(event) => event.currentTarget.select()}
        onChange={(event) => {
          setName(event.target.value);
          setError(null);
        }}
        error={error}
      />
      <div className={styles.renameButtons}>
        <Button variant="quiet" onClick={onClose}>{t('decks.cancel')}</Button>
        <Button variant="decision" type="submit" busy={busy}>{t('decks.rename')}</Button>
      </div>
    </form>
  );
}

/** On servers with accounts: whether the decks are saved to the account. */
function SyncNote() {
  const { status, error } = useDeckSyncStatus();
  const t = useT();
  if (status === 'off') return null;
  const failed = status === 'error';
  return (
    <span className={[styles.sync, failed ? styles.syncFailed : ''].join(' ')} role="status" title={failed ? error ?? undefined : undefined}>
      {failed ? <CloudOff size={15} aria-hidden="true" /> : <Cloud size={15} aria-hidden="true" />}
      {t(status === 'syncing' ? 'decks.sync.saving' : failed ? 'decks.sync.failed' : 'decks.sync.saved')}
    </span>
  );
}
