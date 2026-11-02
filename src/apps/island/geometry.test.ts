import { expect, it } from 'vitest';
import { geometryFor } from './geometry';

it('grows the shelf drop panel with its content, within the peek limit', () => {
  expect(geometryFor('peek', 'drop').h).toBe(220);
  expect(geometryFor('peek', 'drop', '', 430).h).toBe(430);
  expect(geometryFor('compact', 'drop', '', 5000).h).toBeLessThanOrEqual(600);
});
