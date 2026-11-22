import { useQuery } from '@tanstack/react-query';
import { Search } from 'lucide-react';
import { useMemo, useState, type FormEvent } from 'react';
import { formatRules, isCommanderFormat } from '../../core/decks/formats';
import { ALL_META_TIERS, type MetaTier } from '../../core/decks/metaDecks';
import { api } from '../connection';
import { useDecks } from '../stores/decks';
import { useDeckRoster } from '../stores/useDeckRoster';
import { useSharedDecks } from '../stores/sharedDecks';
import { registerMessages, useT } from '../i18n';
import messages from '../i18n/en/career';
import {
  adaptiveSentence, adaptiveFormatFor, ghostOpponentId, ghostRecord, normalizeGhostCode, opponentSource,
} from '../solo/freePlay';
import type { GhostOpponent } from '../../protocol/generated/views';
import { DEFAULT_AI_OPTIONS, usePlay } from '../stores/play';
import { AI_TYPES, DIFFICULTIES, difficultyOf, offeredAiTypes } from './aiSetup';
import { buildAiDeckEntries, countRandomPool } from './aiDeckPool';
import { AiDeckPicker, GHOST_CHOICE } from './AiDeckPicker';
import { useServerState } from '../queries';
import { Button } from '../ui/Button';
import { Dialog } from '../ui/Dialog';
import { Field } from '../ui/Field';
import styles from './AiSettingsDialog.module.css';

registerMessages(messages);

const TIER_KEYS: Record<MetaTier, 'ai.pool.tier.meta' | 'ai.pool.tier.rogue' | 'ai.pool.tier.fun'> = {
  meta: 'ai.pool.tier.meta',
  rogue: 'ai.pool.tier.rogue',
  fun: 'ai.pool.tier.fun',
};

function Segmented<T extends string | number>({ label, value, options, onChange }: {
  label: string;
  value: T;
  options: { value: T; label: string }[];
  onChange(value: T): void;
}) {
  return (
    <div className={styles.segmented} role="radiogroup" aria-label={label}>
      {options.map((option) => (
        <button
          key={String(option.value)}
          type="button"
          role="radio"
          aria-checked={value === option.value}
          className={value === option.value ? styles.segmentOn : styles.segment}
          onClick={() => onChange(option.value)}
        >
          {option.label}
        </button>
      ))}
    </div>
  );
}

/** How the next game against the AI is set up. Changes apply right away and are remembered in this browser. */
export function AiSettingsDialog({ open, onOpenChange }: { open: boolean; onOpenChange(open: boolean): void }) {
  const options = usePlay((state) => state.aiOptions);
  const setOptions = usePlay((state) => state.setAiOptions);
  const decks = useDecks();
  const roster = useDeckRoster();
  // decks other players share on this server (#137)
  const shared = (useSharedDecks(open).data ?? []).filter((deck) => !deck.mine);
  const difficulty = difficultyOf(options.skill);
  const t = useT();
  // a server can leave the Monte Carlo AI out
  const playerTypes = useServerState().data?.playerTypes;
  const source = opponentSource(options.opponentDeckId);
  // the ghost entry is picked before its code is found; until then the saved choice stays as it was
  const [ghostPicked, setGhostPicked] = useState(source.kind === 'ghost');
  const deckValue = ghostPicked ? GHOST_CHOICE : options.opponentDeckId ?? '';
  const allEntries = useMemo(
    () => buildAiDeckEntries({ roster, metaDecks: decks.metaDecks, shared }),
    [roster, decks.metaDecks, shared],
  );
  const commanderEntries = useMemo(
    () => buildAiDeckEntries({ roster, metaDecks: decks.metaDecks, shared, commanderOnly: true }),
    [roster, decks.metaDecks, shared],
  );
  const excludedSet = useMemo(() => new Set(options.excludedDeckIds ?? []), [options.excludedDeckIds]);
  const poolTiers = useMemo(() => options.poolTiers ?? [...ALL_META_TIERS], [options.poolTiers]);
  const poolFormats = useMemo(() => options.poolFormats ?? [], [options.poolFormats]);
  const metaFormats = useMemo(() => [...new Set(decks.metaDecks.map((deck) => deck.format))], [decks.metaDecks]);
  const poolStats = useMemo(
    () => countRandomPool(allEntries, {
      randomPool: options.randomPool,
      poolTiers,
      poolFormats,
      excludedIds: options.excludedDeckIds ?? [],
    }),
    [allEntries, options.randomPool, poolTiers, poolFormats, options.excludedDeckIds],
  );

  function pickDeck(value: string) {
    if (value === GHOST_CHOICE) {
      setGhostPicked(true);
      return;
    }
    setGhostPicked(false);
    setOptions({ opponentDeckId: value || null, opponentLabel: null });
  }

  function toggleExcluded(id: string) {
    const next = new Set(options.excludedDeckIds ?? []);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    setOptions({ excludedDeckIds: [...next] });
  }

  function toggleTier(tier: MetaTier) {
    const next = poolTiers.includes(tier)
      ? poolTiers.filter((item) => item !== tier)
      : [...poolTiers, tier];
    setOptions({ poolTiers: next });
  }

  function togglePoolFormat(format: string) {
    const next = poolFormats.includes(format)
      ? poolFormats.filter((item) => item !== format)
      : [...poolFormats, format];
    setOptions({ poolFormats: next });
  }

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title={t('ai.title')}
      width="md"
      footer={(
        <>
          <Button variant="quiet" onClick={() => {
            setGhostPicked(false);
            setOptions(DEFAULT_AI_OPTIONS);
          }}>{t('ai.restore')}</Button>
          <Button variant="print" onClick={() => onOpenChange(false)}>{t('ai.done')}</Button>
        </>
      )}
    >
      <div className={styles.form}>
        <section className={styles.row}>
          <h3 className={styles.label}>{t('ai.difficulty')}</h3>
          {!options.adaptive && (
            <>
              <Segmented
                label={t('ai.difficulty')}
                value={difficulty.skill}
                options={DIFFICULTIES.map((item) => ({ value: item.skill, label: t(item.label) }))}
                onChange={(skill) => setOptions({ skill })}
              />
              <p className={styles.detail}>{t(difficulty.detail)}</p>
            </>
          )}
          <label className={styles.check}>
            <input type="checkbox" checked={options.adaptive} onChange={(event) => setOptions({ adaptive: event.target.checked })} />
            {t('solo.adaptive.toggle')}
          </label>
          {options.adaptive && <AdaptiveNote open={open} />}
        </section>

        <section className={styles.row}>
          <h3 className={styles.label}>{t(options.opponents > 1 ? 'ai.deck.first' : 'ai.deck')}</h3>
          <AiDeckPicker
            entries={allEntries}
            value={deckValue}
            onChange={pickDeck}
            excludedIds={excludedSet}
            onToggleExclude={toggleExcluded}
          />
          {!ghostPicked && <p className={styles.detail}>{t(source.kind === 'mirror' ? 'solo.mirror.detail' : 'ai.deck.detail')}</p>}
          {ghostPicked && (
            <GhostFinder
              current={source.kind === 'ghost' ? source.code : ''}
              label={options.opponentLabel}
              onFound={(ghost) => setOptions({ opponentDeckId: ghostOpponentId(ghost.code ?? ''), opponentLabel: ghost.name ?? null })}
            />
          )}
        </section>

        <section className={styles.row}>
          <h3 className={styles.label}>{t('ai.pool')}</h3>
          <Segmented
            label={t('ai.pool')}
            value={options.randomPool}
            options={[{ value: 'starters' as const, label: t('ai.pool.starters') }, { value: 'every' as const, label: t('ai.pool.every') }]}
            onChange={(randomPool) => setOptions({ randomPool })}
          />
          <p className={styles.detail}>{t(options.randomPool === 'every' ? 'ai.pool.every.detail' : 'ai.pool.starters.detail')}</p>

          <div className={styles.poolControls}>
            <div className={styles.poolGroup}>
              <span className={styles.subLabel}>{t('ai.pool.tiersLabel')}</span>
              <div className={styles.chipGroup} role="group" aria-label={t('ai.pool.tiersLabel')}>
                {ALL_META_TIERS.map((tier) => {
                  const on = poolTiers.includes(tier);
                  return (
                    <button
                      key={tier}
                      type="button"
                      className={on ? styles.chipOn : styles.chip}
                      aria-pressed={on}
                      onClick={() => toggleTier(tier)}
                    >
                      {t(TIER_KEYS[tier])}
                    </button>
                  );
                })}
              </div>
              <p className={styles.detail}>{t('ai.pool.tiersHint')}</p>
            </div>

            {metaFormats.length > 0 && (
              <div className={styles.poolGroup}>
                <span className={styles.subLabel}>{t('ai.pool.formatsLabel')}</span>
                <div className={styles.chipGroup} role="group" aria-label={t('ai.pool.formatsLabel')}>
                  <button
                    type="button"
                    className={poolFormats.length === 0 ? styles.chipOn : styles.chip}
                    aria-pressed={poolFormats.length === 0}
                    onClick={() => setOptions({ poolFormats: [] })}
                  >
                    {t('ai.picker.allFormats')}
                  </button>
                  {metaFormats.map((format) => {
                    const on = poolFormats.includes(format);
                    return (
                      <button
                        key={format}
                        type="button"
                        className={on ? styles.chipOn : styles.chip}
                        aria-pressed={on}
                        onClick={() => togglePoolFormat(format)}
                      >
                        {formatRules(format).label}
                      </button>
                    );
                  })}
                </div>
              </div>
            )}

            <div className={styles.poolSummary}>
              <span>{t('ai.pool.count', { allowed: poolStats.allowed, total: poolStats.total })}</span>
              {excludedSet.size > 0 && (
                <button
                  type="button"
                  className={styles.clearLink}
                  onClick={() => setOptions({ excludedDeckIds: [] })}
                >
                  {t('ai.pool.clearExcluded', { count: excludedSet.size })}
                </button>
              )}
            </div>
          </div>
        </section>

        <section className={styles.row}>
          <h3 className={styles.label}>{t('ai.type')}</h3>
          <Segmented label={t('ai.type')} value={options.aiType} options={offeredAiTypes(playerTypes).map((item) => ({ value: item.value, label: t(item.label) }))} onChange={(aiType) => setOptions({ aiType })} />
          <p className={styles.detail}>{t(AI_TYPES.find((item) => item.value === options.aiType)?.detail ?? 'ai.type.standard.detail')}</p>
        </section>

        <div className={styles.pair}>
          <section className={styles.row}>
            <h3 className={styles.label}>{t('ai.match')}</h3>
            <Segmented
              label={t('ai.match.length')}
              value={options.winsNeeded}
              options={[{ value: 1 as const, label: t('ai.match.one') }, { value: 2 as const, label: t('ai.match.three') }]}
              onChange={(winsNeeded) => setOptions({ winsNeeded })}
            />
          </section>
          <section className={styles.row}>
            <h3 className={styles.label}>{t('ai.life')}</h3>
            <Segmented
              label={t('ai.life')}
              value={options.startingLife ?? 0}
              options={[{ value: 0, label: t('ai.life.format') }, ...[20, 30, 40].map((life) => ({ value: life, label: String(life) }))]}
              onChange={(life) => setOptions({ startingLife: life === 0 ? null : (life as 20 | 30 | 40) })}
            />
          </section>
        </div>

        <section className={styles.row}>
          <h3 className={styles.label}>{t('ai.rules')}</h3>
          <Segmented
            label={t('ai.rules')}
            value={options.rules}
            options={[{ value: 'casual' as const, label: t('ai.rules.casual') }, { value: 'deck' as const, label: t('ai.rules.deck') }]}
            onChange={(rules) => setOptions({ rules })}
          />
          <p className={styles.detail}>
            {t(options.rules === 'casual' ? 'ai.rules.casual.detail' : 'ai.rules.deck.detail')}
          </p>
        </section>

        <section className={styles.row}>
          <h3 className={styles.label}>{t('ai.pod')}</h3>
          <Segmented
            label={t('ai.pod.label')}
            value={options.opponents}
            options={[{ value: 1 as const, label: t('ai.pod.one') }, { value: 2 as const, label: t('ai.pod.three') }, { value: 3 as const, label: t('ai.pod.four') }]}
            onChange={(opponents) => setOptions({ opponents })}
          />
          <p className={styles.detail}>{t('ai.pod.detail')}</p>
          {/* the pod's other seats; the first is "First AI's deck" above */}
          {POD_SEATS.slice(0, options.opponents - 1).map((seat, index) => (
            <div key={seat} className={styles.row}>
              <h4 className={styles.label}>{t(seat)}</h4>
              <AiDeckPicker
                entries={commanderEntries}
                value={options.podDeckIds[index] ?? ''}
                onChange={(picked) => {
                  const podDeckIds = [...options.podDeckIds];
                  podDeckIds[index] = picked || null;
                  setOptions({ podDeckIds });
                }}
                excludedIds={excludedSet}
                allowSpecial={false}
              />
            </div>
          ))}
          {options.opponents > 1 && <p className={styles.detail}>{t('ai.pod.seatDetail')}</p>}
        </section>
      </div>
    </Dialog>
  );
}

const POD_SEATS = ['ai.pod.seat2', 'ai.pod.seat3'] as const;

/** The adaptive skill's sentence for the selected deck's kind of game (never the number). */
function AdaptiveNote({ open }: { open: boolean }) {
  const t = useT();
  const selectedId = useDecks((state) => state.selectedId);
  const loadList = useDecks((state) => state.loadList);
  const kind = useQuery({
    queryKey: ['deckIsCommander', selectedId],
    queryFn: async () => (selectedId ? isCommanderFormat((await loadList(selectedId)).format) : false),
    enabled: open,
  });
  const format = adaptiveFormatFor(kind.data ?? false);
  const view = useQuery({ queryKey: ['soloAdaptive', format], queryFn: () => api.soloAdaptive(format), enabled: open && kind.isSuccess });
  return (
    <p className={styles.detail} role="status">
      {adaptiveSentence(t, view.data) ?? t(view.isError ? 'solo.adaptive.unavailable' : 'solo.adaptive.detail')}
    </p>
  );
}

/** Find a ghost deck by the code its owner shared, and keep it as the AI's deck. */
function GhostFinder({ current, label, onFound }: { current: string; label: string | null; onFound(ghost: GhostOpponent): void }) {
  const t = useT();
  const [code, setCode] = useState(current);
  const [found, setFound] = useState<GhostOpponent | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function find(event: FormEvent) {
    event.preventDefault();
    const wanted = normalizeGhostCode(code);
    if (!wanted) return;
    setBusy(true);
    setError(null);
    try {
      const ghost = await api.ghostDeck(wanted);
      if (!ghost?.deck) throw new Error();
      setFound(ghost);
      onFound(ghost);
    } catch {
      setFound(null);
      setError(t('solo.ghost.notFound', { code: wanted }));
    } finally {
      setBusy(false);
    }
  }

  const shown = found ?? (current && label ? { code: current, name: label } : null);
  const record = found ? ghostRecord(found) : null;
  return (
    <form className={styles.ghost} onSubmit={(event) => void find(event)}>
      <div className={styles.ghostRow}>
        <Field
          label={t('solo.ghost.code')}
          value={code}
          maxLength={16}
          autoCapitalize="characters"
          spellCheck={false}
          onChange={(event) => {
            setCode(event.target.value);
            setError(null);
          }}
          error={error}
        />
        <Button type="submit" variant="print" icon={<Search size={16} />} busy={busy} disabled={!normalizeGhostCode(code)}>{t('solo.ghost.find')}</Button>
      </div>
      {shown ? (
        <p className={styles.detail} role="status">
          {found?.owner ? t('solo.ghost.foundBy', { name: shown.name ?? shown.code ?? '', owner: found.owner }) : t('solo.ghost.found', { name: shown.name ?? shown.code ?? '' })}
          {record && record.played > 0 && <> · {t('solo.ghost.record', { won: record.won, played: record.played })}</>}
        </p>
      ) : (
        <p className={styles.detail}>{t('solo.ghost.detail')}</p>
      )}
    </form>
  );
}
