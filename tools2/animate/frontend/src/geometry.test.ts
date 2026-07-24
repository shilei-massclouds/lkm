import { describe, expect, it } from 'vitest';
import { signalGeometry, type RectLike } from './geometry';

const stage: RectLike = { left: 10, right: 810, top: 20, bottom: 620, width: 800, height: 600 };
const source: RectLike = { left: 80, right: 280, top: 100, bottom: 200, width: 200, height: 100 };
const target: RectLike = { left: 500, right: 700, top: 260, bottom: 360, width: 200, height: 100 };

function pathNumbers(path: string) {
  return Array.from(path.matchAll(/-?\d+(?:\.\d+)?/g), (match) => Number(match[0]));
}

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
    expect(geometry.path).toMatch(/, 2 284, 74 136$/);
    expect(geometry.labelY).toBeGreaterThan(source.bottom - stage.top);
  });

  it('scales ordinary curvature with endpoint distance and node width', () => {
    const near = pathNumbers(signalGeometry(source, target, stage).path);
    const farTarget = { ...target, left: 900, right: 1100 };
    const far = pathNumbers(signalGeometry(source, farTarget, stage).path);
    const wideSource = { ...source, right: 480, width: 400 };
    const wideTarget = { ...target, left: 700, right: 900, width: 200 };
    const wide = pathNumbers(signalGeometry(wideSource, wideTarget, stage).path);
    expect(far[2] - far[0]).toBeGreaterThan(near[2] - near[0]);
    expect(wide[2] - wide[0]).toBeGreaterThan(near[2] - near[0]);
  });

  it('scales self-loop reach and depth with the node dimensions', () => {
    const regular = pathNumbers(signalGeometry(source, source, stage, 0, 0, true).path);
    const large = { ...source, right: 480, bottom: 300, width: 400, height: 200 };
    const expanded = pathNumbers(signalGeometry(large, large, stage, 0, 0, true).path);
    expect(expanded[2] - expanded[0]).toBeCloseTo((regular[2] - regular[0]) * 2);
    expect(expanded[3] - (large.bottom - stage.top)).toBeGreaterThan(
      regular[3] - (source.bottom - stage.top)
    );
  });
});
