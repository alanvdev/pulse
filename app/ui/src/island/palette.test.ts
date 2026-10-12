import { it, expect } from 'vitest';
import { fuzzy } from './shared';
it('matches letters in order, ignores accents and marks matches', () => {
  expect(fuzzy('Modo foco por 1 hora', 'mfh').ok).toBe(true);
  expect(fuzzy('Silenciar todo', 'xyz').ok).toBe(false);
  expect(fuzzy('Aprobar todos', 'apró').ok).toBe(true);
  expect(fuzzy('Ir', 'i').html).toBe('<b>I</b>r');
});
