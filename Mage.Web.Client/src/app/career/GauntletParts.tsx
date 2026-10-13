import { useQuery } from '@tanstack/react-query';
import { Coins, Copy, Crown, HeartCrack, Landmark, Moon, ShoppingBag, Sparkles, Swords, Skull } from 'lucide-react';
import { useState } from 'react';
import type { CareerGauntletState, CareerRun, CareerRunNode, StartOptions } from '../../protocol/generated/views';
import { api } from '../connection';
import { useT, type MessageKey } from '../i18n';
import { notify } from '../stores/toasts';
import { Button } from '../ui/Button';
import { useDeckChoices, useDeckList } from '../train/trainData';
import { useCareerShop } from './careerData';
import { moveGauntlet, startGauntlet } from './careerModesData';
import styles from './CareerModes.module.css';

function message(reason: unknown): string {
  return reason instanceof Error ? reason.message : String(reason);
}

const NODE_KEYS: Record<string, MessageKey> = {
  fight: 'career.map.fight',
  elite: 'career.map.elite',
  boss: 'career.map.boss',
  shop: 'career.map.shop',
  rest: 'career.map.rest',
  event: 'career.map.event',
};

function NodeIcon({ type, size = 18 }: { type: string | undefined; size?: number }) {
  switch (type) {
    case 'elite':
      return <Skull size={size} aria-hidden="true" />;
    case 'boss':
      return <Crown size={size} aria-hidden="true" />;
    case 'shop':
      return <ShoppingBag size={size} aria-hidden="true" />;
    case 'rest':
      return <Moon size={size} aria-hidden="true" />;
    case 'event':
      return <Sparkles size={size} aria-hidden="true" />;
    default:
      return <Swords size={size} aria-hidden="true" />;
  }
}

/** Starting a run (S022–S028): the mode, the packs, an ascension level and an optional shared seed. */
export function GauntletEntry({ data, coins }: { data: CareerGauntletState; coins: number }) {
  const t = useT();
  const [mode, setMode] = useState('map');
  const [variant, setVariant] = useState('jumpstart');
  const [ascension, setAscension] = useState(0);
  const [seed, setSeed] = useState('');
  const [cube, setCube] = useState('');
  const [cubeDeck, setCubeDeck] = useState('');
  const [setCode, setSetCode] = useState('');
  const [busy, setBusy] = useState(false);
  const price = data.entryCoins ?? 0;
  const short = coins < price;
  const open = data.ascensionOpen ?? 0;
  const cubes = useQuery({ queryKey: ['draftCubes'], queryFn: () => api.getDraftCubes(), enabled: variant === 'cube', staleTime: Infinity });
  const decks = useDeckChoices();
  const chosenDeck = useDeckList(variant === 'cube' && cubeDeck ? cubeDeck : null);
  const shop = useCareerShop(variant === 'draft');
  const sets = (shop.data ?? []).filter((set) => !set.locked);
  const level = (data.ascension ?? []).filter((item) => (item.level ?? 0) <= ascension);
  const cubeReady = variant !== 'cube' || !!cube || !!chosenDeck.deck;

  async function start() {
    setBusy(true);
    const options: StartOptions = { mode, variant, ascension: mode === 'map' ? ascension : 0, seed: seed.trim() || undefined };
    if (variant === 'cube') {
      if (chosenDeck.deck) options.cubeCards = [...chosenDeck.deck.cards, ...(chosenDeck.deck.sideboard ?? [])].map((card) => card.cardName ?? '').filter(Boolean);
      else options.cubeId = cube;
    }
    if (variant === 'draft') options.setCode = setCode || sets[0]?.setCode;
    try {
      await startGauntlet(options);
    } catch (reason) {
      notify(t('career.gauntlet.startFailed'), message(reason), 'error');
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className={styles.panel} aria-labelledby="gauntlet-entry">
      <h2 id="gauntlet-entry" className={styles.panelTitle}>{t('career.gauntlet.newRun')}</h2>
      <fieldset className={styles.fieldset}>
        <legend>{t('career.gauntlet.mode')}</legend>
        <div className={styles.kinds} role="radiogroup" aria-label={t('career.gauntlet.mode')}>
          {(data.modes ?? []).map((item) => (
            <button key={item.id} type="button" role="radio" aria-checked={mode === item.id}
              className={[styles.kind, mode === item.id ? styles.kindOn : ''].join(' ')} onClick={() => setMode(item.id ?? 'map')}>
              <b>{item.name}</b>
              <small>{item.text}</small>
            </button>
          ))}
        </div>
      </fieldset>
      <fieldset className={styles.fieldset}>
        <legend>{t('career.gauntlet.variant')}</legend>
        <div className={styles.kinds} role="radiogroup" aria-label={t('career.gauntlet.variant')}>
          {(data.variants ?? []).map((item) => (
            <button key={item.id} type="button" role="radio" aria-checked={variant === item.id}
              className={[styles.kind, variant === item.id ? styles.kindOn : ''].join(' ')} onClick={() => setVariant(item.id ?? 'jumpstart')}>
              <b>{item.name}</b>
              <small>{item.text}</small>
            </button>
          ))}
        </div>
      </fieldset>
      {variant === 'cube' && (
        <div className={styles.fieldRow}>
          <label className={styles.field}>
            <span>{t('career.gauntlet.cube')}</span>
            <select value={cube} onChange={(event) => { setCube(event.target.value); setCubeDeck(''); }}>
              <option value="">{t('career.gauntlet.cubeNone')}</option>
              {(cubes.data ?? []).map((name) => <option key={name} value={name}>{name}</option>)}
            </select>
          </label>
          <label className={styles.field}>
            <span>{t('career.gauntlet.cubeDeck')}</span>
            <select value={cubeDeck} onChange={(event) => { setCubeDeck(event.target.value); setCube(''); }}>
              <option value="">{t('career.gauntlet.cubeNone')}</option>
              {decks.choices.map((deck) => <option key={deck.id} value={deck.id}>{deck.name}</option>)}
            </select>
          </label>
        </div>
      )}
      {variant === 'draft' && (
        <label className={styles.field}>
          <span>{t('career.limited.set')}</span>
          <select value={setCode || sets[0]?.setCode || ''} onChange={(event) => setSetCode(event.target.value)}>
            {sets.map((set) => <option key={set.setCode} value={set.setCode}>{set.name} ({set.setCode})</option>)}
          </select>
        </label>
      )}
      <div className={styles.fieldRow}>
        {mode === 'map' && (
          <label className={styles.field}>
            <span>{t('career.gauntlet.ascension')}</span>
            <select value={ascension} onChange={(event) => setAscension(Number(event.target.value))} disabled={open === 0}>
              {Array.from({ length: open + 1 }, (_, value) => (
                <option key={value} value={value}>{value === 0 ? t('career.gauntlet.ascensionNone') : t('career.gauntlet.ascensionLevel', { level: value })}</option>
              ))}
            </select>
          </label>
        )}
        <label className={styles.field}>
          <span>{t('career.gauntlet.seed')}</span>
          <input value={seed} onChange={(event) => setSeed(event.target.value)} placeholder="K7Q2-9F3M" maxLength={12} spellCheck={false} />
        </label>
      </div>
      {mode === 'map' && open === 0 && <p className={styles.small}>{t('career.gauntlet.ascensionLocked')}</p>}
      {mode === 'map' && level.length > 0 && (
        <ul className={styles.reasons}>{level.map((item) => <li key={item.level}><b>{item.title}:</b> {item.text}</li>)}</ul>
      )}
      <p className={styles.small}>{t('career.gauntlet.seedHint')}</p>
      <div className={styles.actions}>
        <Button variant="decision" size="lg" icon={<Coins size={18} />} busy={busy} disabled={short || !cubeReady} aria-describedby={short ? 'gauntlet-short' : undefined} onClick={() => void start()}>
          {t('career.gauntlet.start', { price })}
        </Button>
        {short && <span id="gauntlet-short" className={styles.warn}>{t('career.shop.short')}</span>}
      </div>
    </section>
  );
}

/** The run's state at a glance: run coins, wounds, relics, ascension and the seed to share. */
export function RunStatus({ run }: { run: CareerRun }) {
  const t = useT();
  const relics = run.relics ?? [];
  async function copySeed() {
    try {
      await navigator.clipboard.writeText(run.seed ?? '');
      notify(t('career.gauntlet.seedCopied'), run.seed ?? '');
    } catch {
      notify(t('career.gauntlet.seed'), run.seed ?? '');
    }
  }
  return (
    <section className={styles.card} aria-labelledby="run-status">
      <h2 id="run-status" className={styles.cardLabel}>{t('career.gauntlet.status')}</h2>
      <ul className={styles.statusList}>
        <li><Coins size={14} aria-hidden="true" /> {t('career.gauntlet.runCoins', { coins: run.runCoins ?? 0 })}</li>
        {(run.wounds ?? 0) > 0 && <li><HeartCrack size={14} aria-hidden="true" /> {t('career.gauntlet.wounds', { count: run.wounds ?? 0, life: (run.wounds ?? 0) * (run.woundLife ?? 2) })}</li>}
        {run.ascensionTitle && <li><Landmark size={14} aria-hidden="true" /> {run.ascensionTitle}</li>}
        {run.seed && (
          <li>
            {t('career.gauntlet.seedIs', { seed: run.seed })}
            <button type="button" className={styles.stepper} aria-label={t('career.gauntlet.copySeed')} onClick={() => void copySeed()}><Copy size={14} /></button>
          </li>
        )}
      </ul>
      {relics.length > 0 && (
        <>
          <h3 className={styles.subhead}>{t('career.gauntlet.relics')}</h3>
          <ul className={styles.relics}>
            {relics.map((relic) => <li key={relic.id}><b>{relic.name}</b> <small>{relic.text}</small></li>)}
          </ul>
        </>
      )}
    </section>
  );
}

/** The map (S022): rows from the start at the bottom to the last boss at the top; the open nodes are the choices. */
export function RunMap({ run, onFight }: { run: CareerRun; onFight(): void }) {
  const t = useT();
  const [busy, setBusy] = useState<string | null>(null);
  const rows = [...(run.map ?? [])].reverse();
  const here = (run.map ?? []).flat().find((node) => node.id === run.at);
  const fightWaiting = !!run.next && !!here && here.state === 'here';

  async function move(node: CareerRunNode) {
    setBusy(node.id ?? null);
    try {
      await moveGauntlet(node.id ?? '');
    } catch (reason) {
      notify(t('career.map.moveFailed'), message(reason), 'error');
    } finally {
      setBusy(null);
    }
  }

  return (
    <section className={styles.panel} aria-labelledby="gauntlet-map">
      <header className={styles.panelHead}>
        <h2 id="gauntlet-map" className={styles.panelTitle}>{t('career.map.title', { wins: run.wins ?? 0 })}</h2>
        <p className={styles.small}>{fightWaiting ? t('career.map.fightWaiting', { name: run.next?.name ?? '' }) : (run.choices ?? []).length > 0 ? t('career.map.choose') : ''}</p>
      </header>
      {fightWaiting && (
        <div className={styles.actions}>
          <Button variant="decision" size="lg" icon={here?.type === 'boss' ? <Crown size={18} /> : <Swords size={18} />} onClick={onFight} data-nav>
            {t('career.gauntlet.play', { name: run.next?.name ?? '' })}
          </Button>
        </div>
      )}
      <ol className={styles.runMap} aria-label={t('career.map.label')}>
        {rows.map((row, index) => (
          <li key={index} className={styles.mapRow}>
            <ul aria-label={t('career.map.row', { row: (row[0]?.row ?? 0) + 1 })}>
              {row.map((node) => {
                const open = node.state === 'open';
                const label = [
                  t(NODE_KEYS[node.type ?? 'fight'] ?? 'career.map.fight'),
                  node.opponent ? `${node.opponent.name} (${t('career.node.skill', { skill: node.opponent.skill ?? 1 })})` : '',
                  t(`career.map.state.${node.state ?? 'ahead'}` as MessageKey),
                ].filter(Boolean).join(', ');
                return (
                  <li key={node.id}>
                    <button
                      type="button"
                      className={[styles.mapNode, styles[`node_${node.type}`] ?? '', styles[`nodeState_${node.state}`] ?? ''].join(' ')}
                      disabled={!open || busy !== null}
                      aria-busy={busy === node.id || undefined}
                      aria-current={node.state === 'here' ? 'step' : undefined}
                      aria-label={label}
                      title={label}
                      onClick={() => void move(node)}
                    >
                      <NodeIcon type={node.type} />
                      <small>{node.opponent?.name ?? t(NODE_KEYS[node.type ?? 'fight'] ?? 'career.map.fight')}</small>
                    </button>
                  </li>
                );
              })}
            </ul>
          </li>
        ))}
      </ol>
    </section>
  );
}
