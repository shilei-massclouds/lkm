import { describe, expect, it } from 'vitest';
import { signalGeometry, type RectLike } from './geometry';

const stage: RectLike = { left: 10, right: 810, top: 20, bottom: 620, width: 800, height: 600 };
const source: RectLike = { left: 80, right: 280, top: 100, bottom: 200, width: 200, height: 100 };
const target: RectLike = { left: 500, right: 700, top: 260, bottom: 360, width: 200, height: 100 };

describe('Signal arrow geometry', () => {
  it('connects source right to target left for ordinary Signals', () => {
    const geometry = signalGeometry(source, target, stage);
    expect(geometry.self).toBe(false);
    expect(geometry.path).toMatch(/^M 270 130 C /);
    expect(geometry.path).toMatch(/ 490 290$/);
  });

  it('uses a lower semicircle for self Signals', () => {
    const geometry = signalGeometry(source, { ...source }, stage, 4, 6, true);
    expect(geometry.self).toBe(true);
    expect(geometry.path).toMatch(/^M 274 136 C /);
    expect(geometry.path).toMatch(/, 2 248, 74 136$/);
    expect(geometry.labelY).toBeGreaterThan(source.bottom - stage.top);
  });
});
