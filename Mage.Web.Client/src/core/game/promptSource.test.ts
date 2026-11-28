import { describe, expect, test } from 'vitest';
import type { CardView } from '../../protocol/generated/views';
import { promptSourceId, stackSourceId } from './promptSource';
import type { Prompt } from './prompt';

const HYDRA = "<font color='#90EE90' object_id='hydra-2'>Guiding Hydra</font> [b7e]";

describe('promptSourceId (#227)', () => {
  test('a question names the permanent it is asked for in its second line', () => {
    const ask = { kind: 'ask', message: 'Move a +1/+1 counter?', text: 'Move a +1/+1 counter?', options: { secondMessage: HYDRA } } as unknown as Prompt;
    expect(promptSourceId(ask)).toBe('hydra-2');
  });

  test('a payment names what it pays for', () => {
    const pay = { kind: 'playMana', message: "Pay {1}<div style='font-size:11pt'><font object_id='vial-1'>Aether Vial</font> [1a2]</div>", text: '', options: {} } as unknown as Prompt;
    expect(promptSourceId(pay)).toBe('vial-1');
  });

  test('priority and prompts without a source name nothing', () => {
    expect(promptSourceId({ kind: 'priority', message: '', text: '', options: { secondMessage: HYDRA } } as unknown as Prompt)).toBeNull();
    expect(promptSourceId({ kind: 'ask', message: '', text: '', options: { secondMessage: 'Guiding Hydra [b7e]' } } as unknown as Prompt)).toBeNull();
    expect(promptSourceId(null)).toBeNull();
  });
});

test('a stack ability points at its source permanent, a spell at nothing', () => {
  expect(stackSourceId({ id: 'ability-1', sourceCard: { id: 'hydra-1', name: 'Guiding Hydra' } } as CardView)).toBe('hydra-1');
  expect(stackSourceId({ id: 'spell-1', name: 'Lightning Bolt' } as CardView)).toBeNull();
});
