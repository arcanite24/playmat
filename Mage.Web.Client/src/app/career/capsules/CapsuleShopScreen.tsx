import { Coins, Gift, Sparkles } from 'lucide-react';
import { lazy, Suspense, useState, type CSSProperties } from 'react';
import { Link, useParams } from 'react-router-dom';
import type { CareerMachine, CareerMachines, CareerMachineSet, CareerPullResult } from '../../../protocol/generated/views';
import { formatNumber, registerMessages, useI18n, useT } from '../../i18n';
import messages from '../../i18n/en/career';
import { notify } from '../../stores/toasts';
import { useSession } from '../../stores/session';
import { Button } from '../../ui/Button';
import { cardStyleArt } from '../../ui/cardStyles';
import { catalogName, useCatalog } from '../../ui/cosmeticsCatalog';
import { sleeveArtOf } from './capsuleArt';
import { playCareerCue } from '../careerSound';
import { useStill } from '../motion';
import { Cabinet } from './Cabinet';
import { buyTokens, machineItems, machineSize, ownedIn, pull, reveal, setOwned, useMachines } from './capsuleData';
import { readUnopened, saveUnopened } from './unopened';
import { TokenIcon } from './TokenIcon';
import { WearSlot } from './WearStyle';
import styles from './Capsules.module.css';

registerMessages(messages);

// the pull sequence is the heaviest part: it loads with the first pull
const PullSequence = lazy(() => import('./PullSequence'));

const nameOf = (thing: { name?: string; es?: string } | undefined, locale: string) =>
  (locale.startsWith('es') ? thing?.es : undefined) ?? thing?.name ?? '';

/** The capsules still inside a machine, by their sets' colours. */
function capsulesLeft(machine: CareerMachine, owned: ReadonlySet<string>): string[] {
  return machineItems(machine).filter((one) => !owned.has(one.item)).map((one) => one.set.capsule ?? '#888888');
}

/**
 * The capsule shop (M31): Career's gachapon. The storefront lines up the machines (newest wave first) with how many
 * capsules each still holds; a machine's own view is the cabinet, the pull buttons and the collection board of its
 * ten sets. A pull plays out in the pull sequence; whatever it gave joins the board only once it's been opened.
 */
export function CapsuleShopScreen() {
  const t = useT();
  const { machine: machineId } = useParams();
  const shop = useMachines();
  const user = useSession((session) => session.userName.toLowerCase());
  const [pulling, setPulling] = useState<{ result: CareerPullResult; machine: CareerMachine; serial: number } | null>(null);
  const [unopened, setUnopened] = useState(() => readUnopened(user));
  // what the last pull gave lights up on the board as it joins it
  const [flash, setFlash] = useState<ReadonlySet<string>>(new Set());

  if (shop.isPending) return <p className={styles.note}>{t('career.loading')}</p>;
  if (shop.isError || !shop.data) return <p className={styles.note}>{t('capsules.unavailable')}</p>;
  const data = shop.data;
  // capsules pulled but not opened yet stay off the board until they are
  const hidden = new Set((unopened?.result.pulls ?? []).map((one) => one.item ?? ''));
  const owned = new Set((data.owned ?? []).filter((item) => !hidden.has(item)));
  const machine = machineId ? data.machines?.find((one) => one.id === machineId) : undefined;

  const start = (result: CareerPullResult, which: CareerMachine) => {
    saveUnopened(user, result);
    setUnopened({ result });
    setPulling((current) => ({ result, machine: which, serial: (current?.serial ?? 0) + 1 }));
  };
  const finish = () => {
    if (pulling) {
      reveal(pulling.result);
      setFlash(new Set((pulling.result.pulls ?? []).map((one) => one.item ?? '')));
    }
    saveUnopened(user, null);
    setUnopened(null);
    setPulling(null);
  };
  // closed early: the capsules wait, unopened
  const later = () => setPulling(null);

  return (
    <div className={styles.screen}>
      <TokenWallet shop={data} />
      {unopened && !pulling && (
        <UnopenedBanner count={unopened.result.pulls?.length ?? 0} onOpen={() => {
          const which = data.machines?.find((one) => one.id === unopened.result.machine);
          if (which) setPulling({ result: unopened.result, machine: which, serial: 1 });
        }} />
      )}
      {machine
        ? <MachineView machine={machine} shop={data} owned={owned} flash={flash} onPulled={(result) => start(result, machine)} />
        : <Storefront shop={data} owned={owned} />}
      {pulling && (
        <Suspense fallback={null}>
          <PullSequence key={pulling.serial} result={pulling.result} machine={pulling.machine} owned={owned} onDone={finish} onLater={later} />
        </Suspense>
      )}
    </div>
  );
}

/** Tokens to spend, and bundles to buy with coins. */
function TokenWallet({ shop }: { shop: CareerMachines }) {
  const t = useT();
  const [buying, setBuying] = useState<number | null>(null);
  const price = shop.tokenPrice ?? 120;
  const coins = shop.coins ?? 0;
  const buy = async (count: number) => {
    setBuying(count);
    try {
      await buyTokens(count);
      playCareerCue('coins');
      navigator.vibrate?.(12);
    } catch (reason) {
      notify(t('capsules.buyFailed'), reason instanceof Error ? reason.message : String(reason), 'error');
    } finally {
      setBuying(null);
    }
  };
  return (
    <section className={styles.wallet} aria-label={t('capsules.wallet')}>
      <span className={styles.tokens} aria-live="polite">
        <TokenIcon size={34} />
        <b>{formatNumber(shop.tokens ?? 0)}</b>
        <span>{t('capsules.tokens', { count: shop.tokens ?? 0 })}</span>
      </span>
      <span className={styles.buy}>
        {(shop.bundles ?? [1, 5, 10]).map((count) => (
          <Button key={count} variant="quiet" disabled={buying !== null || coins < price * count} onClick={() => void buy(count)}
            aria-label={t('capsules.buyLabel', { count, price: price * count })}>
            <TokenIcon size={16} /> {count} · <Coins size={14} aria-hidden="true" /> {formatNumber(price * count)}
          </Button>
        ))}
      </span>
      <small className={styles.earn}>{t('capsules.earn')}</small>
    </section>
  );
}

function UnopenedBanner({ count, onOpen }: { count: number; onOpen(): void }) {
  const t = useT();
  return (
    <div className={styles.unopened} role="status">
      <Gift size={20} aria-hidden="true" />
      <span>{t('capsules.unopened', { count })}</span>
      <Button onClick={onOpen}>{t('capsules.openThem')}</Button>
    </div>
  );
}

/** Every machine, newest wave first, with how many capsules it still holds. */
function Storefront({ shop, owned }: { shop: CareerMachines; owned: ReadonlySet<string> }) {
  const t = useT();
  const locale = useI18n((state) => state.locale);
  const machines = shop.machines ?? [];
  return (
    <div className={styles.storefront} data-count={machines.length}>
      {machines.map((machine) => {
        const size = machineSize(machine);
        const have = ownedIn(machine, [...owned]);
        const name = nameOf(machine, locale);
        return (
          <Link key={machine.id} to={`/career/capsules/${machine.id}`} className={styles.machineCard} data-nav
            onClick={() => playCareerCue('whoosh')}
            style={{ '--cab-body': machine.cabinet?.body, '--cab-trim': machine.cabinet?.trim } as CSSProperties}>
            <Cabinet seed={machine.id ?? ''} body={machine.cabinet?.body ?? '#7a2c28'} trim={machine.cabinet?.trim ?? '#e3b25a'}
              capsules={capsulesLeft(machine, owned)} label={name} className={styles.storeCabinet} />
            <span className={styles.wave}>{t('capsules.wave', { wave: machine.wave ?? 1 })}</span>
            <b className={styles.machineName}>{name}</b>
            <span className={styles.machineBlurb}>{locale.startsWith('es') ? machine.blurbEs ?? machine.blurb : machine.blurb}</span>
            <span className={styles.left}>
              {have === size ? t('capsules.complete') : t('capsules.left', { left: size - have, total: size })}
            </span>
            <span className={styles.meter}><i style={{ width: `${(have / Math.max(1, size)) * 100}%` }} /></span>
            <span className={styles.enter} aria-hidden="true">{t('capsules.enter')}</span>
          </Link>
        );
      })}
      {machines.length > 0 && <p className={styles.note}>{t('capsules.moreWaves')}</p>}
    </div>
  );
}

/** A machine: its cabinet and pull buttons beside the board of its sets. */
function MachineView({ machine, shop, owned, flash, onPulled }: {
  machine: CareerMachine;
  shop: CareerMachines;
  owned: ReadonlySet<string>;
  flash: ReadonlySet<string>;
  onPulled(result: CareerPullResult): void;
}) {
  const t = useT();
  const locale = useI18n((state) => state.locale);
  const still = useStill();
  const [busy, setBusy] = useState(false);
  const size = machineSize(machine);
  const have = ownedIn(machine, [...owned]);
  const left = size - have;
  const tokens = shop.tokens ?? 0;
  const name = nameOf(machine, locale);
  const tenCount = Math.min(10, left);

  const go = async (count: number) => {
    setBusy(true);
    playCareerCue('tokenIn');
    try {
      onPulled(await pull(machine.id!, count));
    } catch (reason) {
      notify(t('capsules.pullFailed'), reason instanceof Error ? reason.message : String(reason), 'error');
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className={styles.machineView}>
      <div className={styles.machineStage}>
        <Cabinet seed={machine.id ?? ''} body={machine.cabinet?.body ?? '#7a2c28'} trim={machine.cabinet?.trim ?? '#e3b25a'}
          capsules={capsulesLeft(machine, owned)} label={t('capsules.cabinet', { name, left })} className={[styles.bigCabinet, still ? '' : styles.idle].join(' ')} />
        <div className={styles.pulls}>
          {left === 0 ? (
            <p className={styles.completeNote}><Sparkles size={18} aria-hidden="true" /> {t('capsules.machineComplete', { name })}</p>
          ) : (
            <>
              <Button className={styles.pullButton} disabled={busy || tokens < 1} onClick={() => void go(1)} data-nav>
                {t('capsules.pull1')} <span className={styles.cost}><TokenIcon size={16} /> 1</span>
              </Button>
              {tenCount > 1 && (
                <Button variant="quiet" className={styles.pullButton} disabled={busy || tokens < tenCount} onClick={() => void go(tenCount)} data-nav>
                  {t('capsules.pullMany', { count: tenCount })} <span className={styles.cost}><TokenIcon size={16} /> {tenCount}</span>
                </Button>
              )}
              {tokens < 1 && <small className={styles.short}>{t('capsules.noTokens')}</small>}
            </>
          )}
          <small className={styles.odds}>{t('capsules.odds', { left })}</small>
        </div>
      </div>
      <CollectionBoard machine={machine} owned={owned} flash={flash} />
    </div>
  );
}

/** Ten sets of five: owned capsules show their painting, the rest the capsule's colour; each set's sleeve beside it. */
export function CollectionBoard({ machine, owned, flash }: { machine: CareerMachine; owned: ReadonlySet<string>; flash?: ReadonlySet<string> }) {
  const t = useT();
  const locale = useI18n((state) => state.locale);
  const catalog = useCatalog();
  const reward = machine.reward ? catalog.find((item) => item.kind === machine.reward!.kind && item.id === machine.reward!.id) : undefined;
  const title = machine.title ? catalog.find((item) => item.kind === machine.title!.kind && item.id === machine.title!.id) : undefined;
  const have = ownedIn(machine, [...owned]);
  const size = machineSize(machine);
  return (
    <section className={styles.board} aria-label={t('capsules.board', { name: nameOf(machine, locale) })}>
      <header className={styles.boardHead}>
        <b>{t('capsules.collected', { have, total: size })}</b>
        <span>{t('capsules.machineReward', { reward: catalogName(reward, locale), title: catalogName(title, locale) })}</span>
      </header>
      <ol className={styles.sets}>
        {(machine.sets ?? []).map((set) => <SetRow key={set.id} set={set} owned={owned} flash={flash} />)}
      </ol>
    </section>
  );
}

function SetRow({ set, owned, flash }: { set: CareerMachineSet; owned: ReadonlySet<string>; flash?: ReadonlySet<string> }) {
  const t = useT();
  const locale = useI18n((state) => state.locale);
  const count = setOwned(set, [...owned]);
  const done = count === (set.items?.length ?? 5);
  const sleeve = sleeveArtOf(set);
  return (
    <li className={styles.setRow} data-done={done || undefined} style={{ '--capsule': set.capsule } as CSSProperties}>
      <span className={styles.setName}>
        <b>{nameOf(set, locale)}</b>
        <small>{done ? t('capsules.setDone') : t('capsules.setCount', { count, total: set.items?.length ?? 5 })}</small>
      </span>
      <span className={styles.slots}>
        {(set.items ?? []).map((item) => {
          const have = owned.has(item.id ?? '');
          if (have) return <WearSlot key={item.id} card={item.card ?? ''} item={item.id ?? ''} flash={flash?.has(item.id ?? '') ?? false} art={cardStyleArt(item.id ?? '', item.card ?? '')} />;
          return (
            <span key={item.id} className={styles.slot} title={t('capsules.unknown')}>
              <span className={styles.slotCapsule} aria-label={t('capsules.unknown')} />
            </span>
          );
        })}
      </span>
      <span className={styles.setReward} data-have={done || undefined} title={t('capsules.setReward')}>
        {sleeve && <img src={sleeve} alt={t('capsules.setRewardOf', { name: nameOf(set, locale) })} loading="lazy" draggable={false} />}
      </span>
    </li>
  );
}
