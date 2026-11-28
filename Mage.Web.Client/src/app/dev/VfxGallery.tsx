import { useState } from 'react';
import { SIGNATURES, type Ink, type Primitive } from '../match/cardVfx';
import { SignatureVfx } from '../match/SignatureVfx';
import { lifetimeOf, type SignatureEffect } from '../match/signatureShapes';

/**
 * Dev builds only (/dev/vfx): the signature effects on a mock table, to look at and tune them without a game.
 * A stack spot at the right, the caster's plate at the bottom, the opponent's at the top, three cards between.
 */

const WIDTH = 1600;
const HEIGHT = 900;
const STACK = { x: 1380, y: 440 };
const ME = { x: 800, y: 820 };
const THEM = { x: 800, y: 80 };
const CARDS = [{ x: 560, y: 300 }, { x: 800, y: 300 }, { x: 1040, y: 300 }];

const PRIMITIVES: Primitive[] = ['lightning', 'fire', 'beam', 'counter', 'sweep', 'tendrils', 'growth', 'halo', 'bounce', 'insight', 'mill', 'mind', 'ritual', 'summon', 'drain', 'claws'];
const INKS: Ink[] = ['red', 'blue', 'black', 'white', 'green', 'gold', 'void', 'mana'];

let nextKey = 1;

export function VfxGallery() {
  const [effects, setEffects] = useState<SignatureEffect[]>([]);
  const [name, setName] = useState('Lightning Bolt');
  const [ink, setInk] = useState<Ink | ''>('');
  // keep effects on the table (paused with document.getAnimations() for a screenshot)
  const [hold, setHold] = useState(false);
  const play = (fx: Primitive, effectInk: Ink, big: boolean) => {
    const targets = fx === 'counter' ? [{ x: STACK.x, y: STACK.y - 90 }] : fx === 'mill' || fx === 'mind' ? [THEM] : fx === 'lightning' || fx === 'fire' ? [THEM, CARDS[2]] : [CARDS[1]];
    const effect: SignatureEffect = {
      key: nextKey++, fx, ink: effectInk, big, from: STACK, caster: ME, targets,
      spills: fx === 'mill' ? [{ at: THEM, count: big ? 10 : 4 }] : [], width: WIDTH, height: HEIGHT,
    };
    setEffects((current) => [...current, effect]);
    if (!hold) setTimeout(() => setEffects((current) => current.filter((item) => item.key !== effect.key)), lifetimeOf(effect));
  };
  const card = SIGNATURES.get(name);
  return (
    <main style={{ padding: 16, background: '#101512', color: '#eee', minHeight: '100vh', fontFamily: 'system-ui' }}>
      <h1 style={{ fontSize: 20, margin: '0 0 8px' }}>Signature effects</h1>
      <div style={{ display: 'flex', flexWrap: 'wrap', gap: 6, marginBottom: 8 }}>
        <label>Ink <select value={ink} onChange={(event) => setInk(event.target.value as Ink | '')}>
          <option value="">default</option>
          {INKS.map((option) => <option key={option}>{option}</option>)}
        </select></label>
        <label><input type="checkbox" checked={hold} onChange={(event) => setHold(event.target.checked)} /> Hold</label>
        <button type="button" onClick={() => setEffects([])}>Clear</button>
        {PRIMITIVES.map((fx) => <button key={fx} type="button" onClick={() => play(fx, ink || 'red', false)}>{fx}</button>)}
        <label>Card <input list="vfx-cards" value={name} onChange={(event) => setName(event.target.value)} /></label>
        <datalist id="vfx-cards">{[...SIGNATURES.keys()].map((key) => <option key={key} value={key} />)}</datalist>
        <button type="button" disabled={!card} onClick={() => card && play(card.fx, ink || card.ink, !!card.big)}>Cast {card ? `(${card.fx}, ${card.ink}${card.big ? ', big' : ''})` : '(not listed)'}</button>
      </div>
      <div data-testid="vfx-table" style={{ position: 'relative', width: WIDTH, height: HEIGHT, background: 'radial-gradient(#24403f, #0d1716)', borderRadius: 12, overflow: 'hidden' }}>
        {[THEM, ME].map((plate, index) => <span key={index} style={{ position: 'absolute', left: plate.x - 60, top: plate.y - 40, width: 120, height: 80, borderRadius: 40, background: '#3e3a2a' }} />)}
        {CARDS.map((point, index) => <span key={index} style={{ position: 'absolute', left: point.x - 63, top: point.y - 88, width: 126, height: 176, borderRadius: 8, background: '#5a4a3a', border: '2px solid #2a2015' }} />)}
        <span style={{ position: 'absolute', left: STACK.x - 63, top: STACK.y - 88, width: 126, height: 176, borderRadius: 8, background: '#2a3a5a' }} />
        <div style={{ position: 'absolute', inset: 0, pointerEvents: 'none' }}><SignatureVfx effects={effects} /></div>
      </div>
    </main>
  );
}
