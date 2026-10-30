import { useQuery } from '@tanstack/react-query';
import { Check, ClipboardPaste, ExternalLink, FileUp, Link2, TriangleAlert } from 'lucide-react';
import { useEffect, useMemo, useRef, useState, type CSSProperties, type KeyboardEvent } from 'react';
import { useNavigate } from 'react-router-dom';
import { toDeck, type CardIssue, type ImportDraft } from '../../../core/deckImport/draft';
import { alternatesFor } from '../../../core/deckImport/resolve';
import { DECK_SITES, exportLinkFor, siteById, type DeckSite } from '../../../core/deckImport/sites';
import type { CardView } from '../../../protocol/generated/views';
import type { DeckCardInfo } from '../../../core/decks/types';
import { useServerState } from '../../queries';
import { sleeveFor, useDecks, type RosterDeck } from '../../stores/decks';
import { useImportSheet } from '../../stores/importSheet';
import { usePlay } from '../../stores/play';
import { notify } from '../../stores/toasts';
import { Button, ButtonLink } from '../../ui/Button';
import { Dialog } from '../../ui/Dialog';
import { DeckBox } from '../../ui/DeckBox';
import { MatPrint } from '../../ui/MatPrint';
import { legalityLine } from '../legality';
import { useCardImage } from '../../../core/images/useCardImage';
import { useCardInfoStore } from '../cardInfo';
import { deckColors, entryKey, finalizeDeck, groupDeck, manaCurve, printingOf } from '../deckModel';
import { ManaCost } from '../../ui/ManaCost';
import { ManaCurve } from '../ManaCurve';
import { useValidation } from '../useValidation';
import { APP_NAME } from '../../brand';
import { SiteMark } from './SiteMark';
import { PASTE_KEYS } from './text';
import { cardLookup, useImportFlow, type ImportStep } from './useImportFlow';
import styles from './ImportSheet.module.css';


const shortFormat = (format: string) => format.replace(/^Constructed - /, '').replace(/^Variant Magic - /, '');

/** The one place decks come in: a link from a deck website, a pasted list or a deck file. */
export function ImportSheet() {
  const open = useImportSheet((state) => state.open);
  const session = useImportSheet((state) => state.session);
  const seed = useImportSheet((state) => state.seed);
  const target = useImportSheet((state) => state.target);
  const close = useImportSheet((state) => state.close);
  const flow = useImportFlow();
  const { seed: start, step, backToInput } = flow;

  // every open starts from its seed (a paste, a drop, a bookmarklet deck, or nothing)
  const seeded = useRef(0);
  useEffect(() => {
    if (!open || seeded.current === session) return;
    seeded.current = session;
    start(seed);
  }, [open, session, seed, start]);

  const title = target.kind === 'into'
    ? (target.mode === 'replace' ? `Replace ${target.deckName}` : `Add to ${target.deckName}`)
    : 'Import a deck';

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => !next && close()}
      title={title}
      width="xl"
      // Esc and the arrow step back to the input before the sheet closes; reading a deck has Cancel instead
      onBack={step.kind === 'assisted' || step.kind === 'preview' ? backToInput : undefined}
    >
      <StepView flow={flow} onDone={close} />
    </Dialog>
  );
}

type Flow = ReturnType<typeof useImportFlow>;

function StepView({ flow, onDone }: { flow: Flow; onDone(): void }) {
  switch (flow.step.kind) {
    case 'input':
      return <InputStep flow={flow} failure={flow.step.failure} />;
    case 'loading':
      return <LoadingStep site={flow.step.site ? siteById(flow.step.site) : undefined} onCancel={flow.cancel} />;
    case 'assisted':
      return <AssistedStep flow={flow} step={flow.step} />;
    case 'preview':
      return <PreviewStep flow={flow} draft={flow.step.draft} onDone={onDone} />;
  }
}

function InputStep({ flow, failure }: { flow: Flow; failure?: string }) {
  const field = useRef<HTMLTextAreaElement>(null);
  const fileInput = useRef<HTMLInputElement>(null);
  const [dragging, setDragging] = useState(false);
  const { recognized, degraded } = flow;
  const site = recognized.kind === 'site' ? siteById(recognized.site) : undefined;
  const direct = site && site.tier === 'direct' && !degraded.has(site.id);

  // back from a step: the last input is selected, so a new paste replaces it
  useEffect(() => {
    // after whatever opened the sheet (a menu, a button) has finished moving focus
    const frame = requestAnimationFrame(() => {
      field.current?.focus();
      field.current?.select();
    });
    return () => cancelAnimationFrame(frame);
  }, []);

  async function readFile(file: File) {
    if (file.size > 2 * 1024 * 1024) {
      notify(`${file.name} is too large`, 'Deck files are a few kilobytes; this one is more than 2 MB.', 'error');
      return;
    }
    flow.seed({ kind: 'file', name: file.name, text: await file.text() });
  }

  return (
    <div
      className={[styles.body, styles.inputBody, dragging ? styles.dropping : ''].join(' ')}
      onDragOver={(event) => {
        event.preventDefault();
        setDragging(true);
      }}
      onDragLeave={() => setDragging(false)}
      onDrop={(event) => {
        event.preventDefault();
        setDragging(false);
        const file = event.dataTransfer.files?.[0];
        if (file) void readFile(file);
        else {
          const dropped = event.dataTransfer.getData('text/uri-list').split(/\r?\n/).find((line) => line && !line.startsWith('#')) || event.dataTransfer.getData('text/plain');
          if (dropped) {
            flow.setText(dropped);
            void flow.submit(dropped);
          }
        }
      }}
    >
      <label className={styles.fieldLabel} htmlFor="import-input">Deck link or list</label>
      <div className={styles.inputWrap}>
        <textarea
          id="import-input"
          ref={field}
          className={styles.input}
          value={flow.text}
          spellCheck={false}
          placeholder={'https://archidekt.com/decks/…\n\nor a list:\n4 Lightning Bolt\n20 Mountain\n\nSideboard\n2 Smash to Smithereens'}
          aria-describedby="import-hint"
          aria-invalid={failure ? true : undefined}
          onChange={(event) => flow.setText(event.target.value)}
          onPaste={(event) => {
            // a pasted link or list goes straight through, no extra click
            const pasted = event.clipboardData.getData('text/plain');
            const element = event.currentTarget;
            const replacesAll = !flow.text.trim() || (element.selectionStart === 0 && element.selectionEnd === element.value.length);
            if (replacesAll && pasted.trim()) {
              event.preventDefault();
              flow.setText(pasted);
              void flow.submit(pasted);
            }
          }}
          onKeyDown={(event) => {
            if (event.key === 'Enter' && (event.metaKey || event.ctrlKey || recognized.kind === 'site' || recognized.kind === 'unknownUrl')) {
              event.preventDefault();
              void flow.submit();
            }
          }}
        />
        {site && (
          <div className={styles.recognized} role="status">
            <SiteMark site={site} />
            <span><b>{site.name} deck</b> · {direct ? 'reads in one step' : 'copy and paste'}</span>
          </div>
        )}
        {recognized.kind === 'list' && (
          <div className={styles.recognized} role="status">
            <ClipboardPaste size={16} aria-hidden="true" />
            <span><b>Deck list</b> · {recognized.text.split(/\r?\n/).filter((line) => line.trim()).length} lines</span>
          </div>
        )}
      </div>
      <p id="import-hint" className={failure ? styles.failure : styles.hint} role={failure ? 'alert' : undefined}>
        {failure ?? `Paste a link from a deck site, or a list with one card per line, like “4 Lightning Bolt”. Lists from Arena, MTGO and XMage work too. You can also drop a deck file here.`}
      </p>

      <p className={styles.sitesLegend}>Sites marked <Link2 size={12} aria-hidden="true" className={styles.siteTier} /> read in one step. The others take a copy and a paste.</p>
      <div className={styles.sites} aria-label={`Deck sites ${APP_NAME} reads`}>
        {DECK_SITES.map((entry) => {
          const lit = site?.id === entry.id;
          const oneStep = entry.tier === 'direct' && !degraded.has(entry.id);
          return (
            <span key={entry.id} className={[styles.site, lit ? styles.siteLit : ''].join(' ')}>
              <SiteMark site={entry} size="sm" />
              <span className={styles.siteName}>{entry.name}</span>
              {oneStep && <Link2 size={12} aria-label="reads in one step" className={styles.siteTier} />}
            </span>
          );
        })}
      </div>

      <footer className={styles.foot}>
        <Button variant="quiet" icon={<FileUp size={16} />} onClick={() => fileInput.current?.click()}>Choose a file</Button>
        <input
          ref={fileInput}
          type="file"
          accept=".dck,.dec,.txt,.mwdeck,.cod,.o8d,.json,.draft,.dek,.mtga"
          hidden
          onChange={(event) => {
            const file = event.target.files?.[0];
            if (file) void readFile(file);
            event.target.value = '';
          }}
        />
        <Button variant="decision" disabled={recognized.kind === 'empty'} onClick={() => void flow.submit()}>
          {recognized.kind === 'site' && direct ? `Read from ${site!.name}` : 'Continue'}
        </Button>
      </footer>
    </div>
  );
}

function LoadingStep({ site, onCancel }: { site?: DeckSite; onCancel(): void }) {
  return (
    <div className={[styles.body, styles.loadingBody].join(' ')} role="status" aria-live="polite">
      <div className={styles.ghostBox} aria-hidden="true">
        <span className={styles.ghostWindow}>{site && <SiteMark site={site} size="lg" />}</span>
        <span className={styles.ghostPlate}><i /><i /></span>
      </div>
      <p className={styles.loadingText}>{site ? `Reading the deck from ${site.name}…` : 'Matching the cards…'}</p>
      {/* focus waits on Cancel, not on the sheet's outline, while the deck is read */}
      <Button variant="quiet" onClick={onCancel} autoFocus>Cancel</Button>
    </div>
  );
}

function AssistedStep({ flow, step }: { flow: Flow; step: Extract<ImportStep, { kind: 'assisted' }> }) {
  const site = siteById(step.site)!;
  const [opened, setOpened] = useState(false);
  const [back, setBack] = useState(false);
  const [hint, setHint] = useState<string | null>(null);
  const target = useRef<HTMLDivElement>(null);
  const exportUrl = exportLinkFor(site, step.url, step.remoteId);

  // when the player comes back from the site, the paste target is waiting for them
  useEffect(() => {
    if (!opened) return;
    function onFocus() {
      setBack(true);
      target.current?.focus();
    }
    window.addEventListener('focus', onFocus);
    return () => window.removeEventListener('focus', onFocus);
  }, [opened]);

  useEffect(() => {
    function onPaste(event: ClipboardEvent) {
      const text = event.clipboardData?.getData('text/plain') ?? '';
      event.preventDefault();
      void flow.finishAssisted(text).then((done) => {
        if (!done) setHint(/^\s*https?:\/\//.test(text) ? 'That’s the link again. Copy the deck list itself from the export.' : 'That doesn’t look like a deck list. Copy the whole export and paste again.');
      });
    }
    document.addEventListener('paste', onPaste);
    return () => document.removeEventListener('paste', onPaste);
  }, [flow]);

  return (
    <div className={[styles.body, styles.assistedBody].join(' ')}>
      <div className={styles.assistedHead}>
        <SiteMark site={site} size="lg" />
        <div>
          <h3 className={styles.assistedTitle}>Bring it over from {site.name}</h3>
          <p className={styles.assistedNote}>
            {step.failure ? step.failure.message : `${site.name} doesn’t let other apps read its decks, so this one takes a copy and a paste.`}
          </p>
        </div>
      </div>
      <ol className={styles.steps}>
        <li className={opened ? styles.stepDone : ''}>
          <span className={styles.stepNo}>{opened ? <Check size={14} /> : 1}</span>
          <div>
            <ButtonLink href={exportUrl} target="_blank" rel="noopener noreferrer" icon={<ExternalLink size={15} />} onClick={() => setOpened(true)}>
              Open the deck on {site.name}
            </ButtonLink>
          </div>
        </li>
        <li>
          <span className={styles.stepNo}>2</span>
          <div>{site.exportHint}</div>
        </li>
        <li>
          <span className={styles.stepNo}>3</span>
          <div>Come back here and press <kbd className={styles.kbd}>{PASTE_KEYS}</kbd>.</div>
        </li>
      </ol>
      <div
        ref={target}
        tabIndex={0}
        className={[styles.pasteTarget, back ? styles.pasteWaiting : ''].join(' ')}
        aria-label={`Paste the ${site.name} list here`}
      >
        <ClipboardPaste size={22} aria-hidden="true" />
        <span>{back ? `Press ${PASTE_KEYS} to finish` : 'The list lands here'}</span>
      </div>
      {hint && <p className={styles.failure} role="alert">{hint}</p>}
      <p className={styles.hint}>Tip: the “Send to {APP_NAME}” bookmark in Settings brings decks over in one click.</p>
    </div>
  );
}

function FixRow({ issue, onPick, onSkip, autoFocus }: { issue: CardIssue; onPick(card: CardView): void; onSkip(): void; autoFocus: boolean }) {
  const [open, setOpen] = useState(false);
  const button = useRef<HTMLButtonElement>(null);
  const options = useQuery({
    queryKey: ['importAlternates', issue.cardName],
    queryFn: () => alternatesFor({ cardName: issue.cardName }, cardLookup, 12),
    enabled: open,
    staleTime: 5 * 60_000,
  });
  useEffect(() => {
    if (autoFocus) button.current?.focus();
  }, [autoFocus]);

  return (
    <li className={styles.fix}>
      <div className={styles.fixHead}>
        <span className={styles.fixCount}>{issue.amount}</span>
        <span className={styles.fixName}>
          {issue.cardName}
          {issue.setCode && <small> {issue.setCode}{issue.cardNumber ? ` ${issue.cardNumber}` : ''}</small>}
          <small className={styles.fixReason}>{issue.reason}</small>
        </span>
        <Button ref={button} size="sm" variant="decision" aria-expanded={open} onClick={() => setOpen((value) => !value)}>
          {open ? 'Hide' : 'Find it'}
        </Button>
        <Button size="sm" variant="quiet" onClick={onSkip}>Leave out</Button>
      </div>
      {open && (
        <div className={styles.fixOptions}>
          {options.isLoading && <p className={styles.hint}>Looking for close matches…</p>}
          {options.data && options.data.length === 0 && <p className={styles.hint}>Nothing close in the card database. Leave it out, or add it in the builder later.</p>}
          {options.data && options.data.length > 0 && (
            <ul className={styles.optionList}>
              {options.data.map((card) => (
                <li key={`${card.expansionSetCode}:${card.cardNumber}`}>
                  <PrintingOption card={card} onPick={() => onPick(card)} />
                </li>
              ))}
            </ul>
          )}
        </div>
      )}
    </li>
  );
}

/** One printing to pick: its art, name and set. */
function PrintingOption({ card, onPick }: { card: CardView; onPick(): void }) {
  const art = useCardImage({ name: card.name, setCode: card.expansionSetCode ?? '', cardNumber: card.cardNumber ?? '' }, 'front', 'art_crop');
  return (
    <button type="button" className={styles.option} onClick={onPick}>
      <span className={styles.optionArt} style={typeof art === 'string' ? { backgroundImage: `url("${art}")` } : undefined} aria-hidden="true" />
      <span className={styles.optionText}>
        <b>{card.name}</b>
        <span>{card.expansionSetCode} {card.cardNumber}</span>
      </span>
    </button>
  );
}

const COMMANDER_FORMATS = /^Variant Magic - (Commander|Duel Commander|MTGO 1v1 Commander|Brawl|Oathbreaker|Freeform Commander|Penny Dreadful Commander|Tiny Leaders|Centurion Commander)$/;

function PreviewStep({ flow, draft, onDone }: { flow: Flow; draft: ImportDraft; onDone(): void }) {
  const navigate = useNavigate();
  const target = useImportSheet((state) => state.target);
  const sleeves = useDecks((state) => state.sleeves);
  const server = useServerState();
  const [busy, setBusy] = useState(false);
  const { deck, counts } = useMemo(() => toDeck(draft), [draft]);
  const info = useMemo(() => {
    const map = new Map<string, CardView>();
    for (const card of draft.cards.values()) if (card) map.set(entryKey(printingOf(card)), card);
    return map;
  }, [draft.cards]);
  const finished = useMemo(() => finalizeDeck(deck, info), [deck, info]);
  const curve = useMemo(() => manaCurve(deck.cards, info), [deck.cards, info]);
  const colors = useMemo(() => deckColors([...deck.cards, ...deck.sideboard], info), [deck, info]);
  const into = target.kind === 'into' ? target : null;
  // cards going into another deck aren't a deck of their own: no name, format or legality for them
  const validated = useMemo(() => (into ? { ...deck, cards: [] } : deck), [into, deck]);
  const validation = useValidation(validated, draft.format);
  const issues = flow.issues;
  const site = siteById(draft.source?.site);

  const formats = useMemo(() => {
    const all = (server.data?.deckTypes ?? []).filter((type) => type.startsWith('Constructed - ') || COMMANDER_FORMATS.test(type));
    return all.includes(draft.format) ? all : [draft.format, ...all];
  }, [server.data, draft.format]);

  const roster: RosterDeck = {
    id: 'import-preview',
    name: draft.name || 'Imported deck',
    note: `${shortFormat(draft.format)} · ${counts.main} cards${counts.side ? ` + ${counts.side}` : ''}`,
    cardCount: counts.main,
    cover: finished.coverCard ?? null,
    colors,
    starter: null,
  };

  async function save(then: 'stay' | 'play' | 'build') {
    if (counts.main === 0) return;
    setBusy(true);
    try {
      useCardInfoStore.getState().remember([...info.values()]);
      if (target.kind === 'into') {
        target.apply(finished);
        onDone();
        notify(target.mode === 'replace' ? 'Deck replaced' : 'Cards added', `${counts.main} cards from ${site?.name ?? 'the list'}.`);
        return;
      }
      const id = await useDecks.getState().saveImported(finished);
      useImportSheet.getState().markArrived(id);
      onDone();
      notify('Deck imported', `${finished.name} is on your shelf.`, 'info', {
        label: 'Undo',
        run: () => void useDecks.getState().remove(id),
      });
      if (then === 'play') {
        navigate('/');
        void usePlay.getState().playVsAi(id);
      } else if (then === 'build') {
        navigate(`/decks/${encodeURIComponent(id)}`);
      } else {
        navigate('/decks');
      }
    } catch (reason) {
      notify('Couldn’t save the deck', reason instanceof Error ? reason.message : String(reason), 'error');
    } finally {
      setBusy(false);
    }
  }

  function onKeyDown(event: KeyboardEvent) {
    const element = event.target as HTMLElement;
    if (event.key === 'Enter' && !/^(TEXTAREA|SELECT|BUTTON|A)$/.test(element.tagName)) {
      event.preventDefault();
      void save('stay');
    }
  }

  const problems = validation.data?.errors ?? [];
  const playable = problems.length === 0;
  const ready = issues.length === 0 && playable;
  const problemList = useRef<HTMLUListElement>(null);
  const sleeve = sleeveFor(sleeves, roster);

  // a clean import is ready to save: the name takes focus, Enter saves; otherwise the first fix does (FixRow)
  const nameField = useRef<HTMLInputElement>(null);
  const cleanOnArrival = useRef(issues.length === 0);
  useEffect(() => {
    if (cleanOnArrival.current) nameField.current?.focus();
  }, []);

  return (
    <div className={[styles.body, styles.previewBody].join(' ')} onKeyDown={onKeyDown}>
      <section className={styles.deckSide} aria-label="The deck as it lands on your shelf">
        {!into && <MatPrint card={finished.coverCard ?? null} className={styles.deckPrint} />}
        {!into && (
          <div className={styles.boxStage} style={{ '--sleeve': sleeve } as CSSProperties}>
            <DeckBox deck={roster} sleeve={sleeve} />
          </div>
        )}
        <dl className={styles.counts}>
          <div><dt>Deck</dt><dd>{counts.main}</dd></div>
          {counts.commanders > 0 && <div><dt>{counts.commanders > 1 ? 'Commanders' : 'Commander'}</dt><dd>{counts.commanders}</dd></div>}
          {counts.side > 0 && <div><dt>Sideboard</dt><dd>{counts.side}</dd></div>}
        </dl>
        {counts.commanders > 0 && (
          <div className={styles.commanders}>
            {deck.sideboard.map((card) => <span key={`${card.setCode}:${card.cardNumber}`} className={styles.commander}>{card.cardName}</span>)}
          </div>
        )}
        <ManaCurve curve={curve} className={styles.curve} />
        {!into && problems.length > 0 && (
          <ul ref={problemList} tabIndex={-1} className={styles.problems} aria-label={`${shortFormat(draft.format)} problems`}>
            {problems.slice(0, 5).map((problem, index) => <li key={index}>{legalityLine(problem.group, problem.message, draft.format)}</li>)}
            {problems.length > 5 && <li>and {problems.length - 5} more</li>}
          </ul>
        )}
      </section>

      <section className={styles.detailSide}>
        {into ? (
          <p className={styles.intoNote}>
            {into.mode === 'replace'
              ? <>These cards take the place of everything in <b>{into.deckName}</b>. Its name and sleeve stay.</>
              : <>These cards join <b>{into.deckName}</b>.</>}
          </p>
        ) : (
        <>
        <div className={styles.nameRow}>
          <label className={styles.fieldLabel} htmlFor="import-name">Deck name</label>
          <input
            id="import-name"
            ref={nameField}
            className={styles.nameInput}
            value={draft.name}
            maxLength={60}
            onChange={(event) => {
              const name = event.target.value;
              flow.updateDraft((current) => ({ ...current, name }));
            }}
          />
          {draft.source && site && (
            <p className={styles.source}>
              From <a href={draft.source.url} target="_blank" rel="noopener noreferrer">{site.name}</a>
              {draft.author && <> · by {draft.author}</>}
              {draft.description && <> · {draft.description}</>}
            </p>
          )}
        </div>

        <div className={styles.formatRow}>
          <label className={styles.fieldLabel} htmlFor="import-format">Format</label>
          <div className={styles.formatLine}>
            <select
              id="import-format"
              className={styles.select}
              value={draft.format}
              onChange={(event) => {
                const format = event.target.value;
                flow.updateDraft((current) => ({ ...current, format }));
              }}
            >
              {formats.map((type) => <option key={type} value={type}>{shortFormat(type)}</option>)}
            </select>
            {validation.data && (validation.data.valid
              ? <span className={[styles.chip, styles.chipOk].join(' ')}><Check size={14} aria-hidden="true" /> Legal</span>
              : (
                <button type="button" className={[styles.chip, styles.chipBad].join(' ')} onClick={() => problemList.current?.focus()}>
                  <TriangleAlert size={14} aria-hidden="true" /> {problems.length} {problems.length === 1 ? 'problem' : 'problems'}
                </button>
              ))}
          </div>
        </div>
        </>
        )}

        <fieldset className={styles.printings}>
          <legend className={styles.fieldLabel}>Printings</legend>
          <label>
            <input type="radio" name="printings" checked={draft.printings === 'site'} onChange={() => void flow.setPrintings('site')} />
            {draft.source && site ? `Keep ${site.name}’s` : 'Keep the list’s'}
          </label>
          <label>
            <input type="radio" name="printings" checked={draft.printings === 'preferred'} onChange={() => void flow.setPrintings('preferred')} />
            Use my preferred art
          </label>
        </fieldset>

        <div className={styles.fixes} aria-live="polite">
          {issues.length > 0 ? (
            <>
              <h3 className={styles.fixesTitle}>
                {issues.length === 1 ? '1 card needs a look' : `${issues.length} cards need a look`}
              </h3>
              <ul className={styles.fixList}>
                {issues.map((issue, index) => (
                  <FixRow
                    key={issue.key}
                    issue={issue}
                    autoFocus={index === 0}
                    onPick={(card) => flow.pick(issue.key, card, issue)}
                    onSkip={() => flow.skip(issue.key)}
                  />
                ))}
              </ul>
            </>
          ) : (
            <p className={styles.allMatched}><Check size={16} aria-hidden="true" /> Every card matched.</p>
          )}
          {draft.list.skipped.length > 0 && (
            <div className={styles.skipped}>
              <h3 className={styles.fixesTitle}>
                {draft.list.skipped.length === 1 ? '1 line couldn’t be read' : `${draft.list.skipped.length} lines couldn’t be read`}
              </h3>
              <ul className={styles.notes}>
                {draft.list.skipped.map((line, index) => <li key={`${index}:${line}`}><code>{line}</code></li>)}
              </ul>
              <p className={styles.hint}>These aren’t in the deck. Cards are read as a count, then a name: “4 Lightning Bolt”.</p>
            </div>
          )}
          {(counts.maybeboard > 0 || counts.sideboardDropped > 0 || counts.missing > 0) && (
            <ul className={styles.notes}>
              {counts.maybeboard > 0 && <li>{counts.maybeboard} maybeboard {counts.maybeboard === 1 ? 'card stays' : 'cards stay'} behind.</li>}
              {counts.sideboardDropped > 0 && <li>{counts.sideboardDropped} sideboard {counts.sideboardDropped === 1 ? 'card stays' : 'cards stay'} behind: a commander deck keeps only its commanders there.</li>}
              {counts.missing > 0 && issues.length === 0 && <li>{counts.missing} {counts.missing === 1 ? 'card is' : 'cards are'} left out.</li>}
            </ul>
          )}
        </div>

        <DeckList deck={deck} info={info} commanderDeck={counts.commanders > 0} />
      </section>

      <footer className={[styles.foot, styles.previewFoot].join(' ')}>
        <span className={styles.footNote}>
          {issues.length > 0
            ? 'Saving now leaves out the cards that need a look.'
            : !into && !playable ? `Fix ${problems.length === 1 ? 'the problem' : `the ${problems.length} problems`} in the builder to play this deck.` : ''}
        </span>
        {target.kind === 'into' ? (
          <Button variant="decision" busy={busy} disabled={counts.main === 0} onClick={() => void save('stay')}>
            {target.mode === 'replace' ? 'Replace deck' : 'Add cards'}
          </Button>
        ) : (
          <>
            <Button variant="quiet" disabled={busy || counts.main === 0} onClick={() => void save('build')}>Open in builder</Button>
            {/* one amber at most: Save and play when the deck is ready, Save when it can't be played yet,
                and none while cards need a look (Find it is the decision then) */}
            <Button variant={issues.length === 0 && !playable ? 'decision' : 'print'} disabled={busy || counts.main === 0} onClick={() => void save('stay')}>Save</Button>
            <Button variant={ready ? 'decision' : 'print'} busy={busy} disabled={counts.main === 0 || !playable} onClick={() => void save('play')}>Save and play</Button>
          </>
        )}
      </footer>
    </div>
  );
}

/** What's coming in, grouped the way the deck builder groups it. */
function DeckList({ deck, info, commanderDeck }: { deck: { cards: DeckCardInfo[]; sideboard: DeckCardInfo[] }; info: ReadonlyMap<string, CardView>; commanderDeck: boolean }) {
  const groups = useMemo(() => groupDeck(deck.cards, info), [deck.cards, info]);
  const side = useMemo(() => groupDeck(deck.sideboard, info).flatMap((group) => group.rows), [deck.sideboard, info]);
  return (
    <div className={styles.list} aria-label="Cards in the deck">
      {groups.map((group) => (
        <section key={group.key} className={styles.group}>
          <h4 className={styles.groupHead}>{group.label}<span>{group.count}</span></h4>
          <ul>
            {group.rows.map((row) => (
              <li key={row.key} className={styles.row}>
                <span className={styles.rowCount}>{row.entry.amount}</span>
                <span className={styles.rowName}>{row.entry.cardName}</span>
                {row.card && <ManaCost cost={row.card.manaCostLeftStr} size="sm" />}
              </li>
            ))}
          </ul>
        </section>
      ))}
      {!commanderDeck && side.length > 0 && (
        <section className={styles.group}>
          <h4 className={styles.groupHead}>Sideboard<span>{side.reduce((sum, row) => sum + row.entry.amount, 0)}</span></h4>
          <ul>
            {side.map((row) => (
              <li key={row.key} className={styles.row}>
                <span className={styles.rowCount}>{row.entry.amount}</span>
                <span className={styles.rowName}>{row.entry.cardName}</span>
                {row.card && <ManaCost cost={row.card.manaCostLeftStr} size="sm" />}
              </li>
            ))}
          </ul>
        </section>
      )}
    </div>
  );
}
