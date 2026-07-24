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
    expect(geometry.direction).toBe('right');
    expect(geometry.path).toBe('M 270 130 L 490 290');
    expect(geometry.labelX).toBe(380);
    expect(geometry.labelY).toBe(210);
  });

  it('selects opposing edges in all four directions', () => {
    const leftTarget = { ...target, left: -300, right: -100, top: 100, bottom: 200 };
    const downTarget = { ...target, left: 80, right: 280, top: 500, bottom: 600 };
    const upTarget = { ...target, left: 80, right: 280, top: -300, bottom: -200 };
    const right = pathNumbers(signalGeometry(source, target, stage).path);
    const left = signalGeometry(source, leftTarget, stage);
    const down = signalGeometry(source, downTarget, stage);
    const up = signalGeometry(source, upTarget, stage);

    expect(right.slice(0, 2)).toEqual([source.right - stage.left, 130]);
    expect(left.direction).toBe('left');
    expect(pathNumbers(left.path).slice(0, 2)).toEqual([source.left - stage.left, 130]);
    expect(pathNumbers(left.path).slice(-2)).toEqual([leftTarget.right - stage.left, 130]);
    expect(down.direction).toBe('down');
    expect(pathNumbers(down.path).slice(0, 2)).toEqual([170, source.bottom - stage.top]);
    expect(pathNumbers(down.path).slice(-2)).toEqual([170, downTarget.top - stage.top]);
    expect(up.direction).toBe('up');
    expect(pathNumbers(up.path).slice(0, 2)).toEqual([170, source.top - stage.top]);
    expect(pathNumbers(up.path).slice(-2)).toEqual([170, upTarget.bottom - stage.top]);
    for (const geometry of [signalGeometry(source, target, stage), left, down, up]) {
      expect(geometry.path.match(/\bL\b/g)).toHaveLength(1);
      expect(geometry.path).not.toContain(' C ');
      const points = pathNumbers(geometry.path);
      expect(geometry.labelX).toBe((points[0] + points[2]) / 2);
      expect(geometry.labelY).toBe((points[1] + points[3]) / 2);
    }
  });

  it('uses an upper semicircle for self Signals', () => {
    const geometry = signalGeometry(source, { ...source }, stage, 4, 6, true);
    const points = pathNumbers(geometry.path);
    const nodeTop = source.top - stage.top + 6;
    expect(geometry.self).toBe(true);
    expect(geometry.path).toBe('M 274 136 C 346 -12, 2 -12, 74 136');
    expect(points[0]).toBe(source.right - stage.left + 4);
    expect(points[6]).toBe(source.left - stage.left + 4);
    expect(points[1]).toBe(points[7]);
    expect(points[3]).toBeLessThan(nodeTop);
    expect(points[5]).toBeLessThan(nodeTop);
    const curveApex = points[1] * 0.25 + points[3] * 0.75;
    expect(geometry.labelY).toBeLessThan(curveApex);
  });

  it('scales self-loop reach and depth with the node dimensions', () => {
    const regular = pathNumbers(signalGeometry(source, source, stage, 0, 0, true).path);
    const large = { ...source, right: 480, bottom: 300, width: 400, height: 200 };
    const expanded = pathNumbers(signalGeometry(large, large, stage, 0, 0, true).path);
    expect(expanded[2] - expanded[0]).toBeCloseTo((regular[2] - regular[0]) * 2);
    expect((large.top - stage.top) - expanded[3]).toBeGreaterThan(
      (source.top - stage.top) - regular[3]
    );
  });

  it('keeps a scaled self-loop and its label inside the reserved top clearance', () => {
    const clearance = 160;
    const large = {
      left: 80, right: 480, top: stage.top + clearance, bottom: stage.top + clearance + 200,
      width: 400, height: 200
    };
    const geometry = signalGeometry(large, large, stage, 0, 0, true);
    expect(geometry.labelY).toBeGreaterThanOrEqual(0);
    expect(geometry.labelY).toBeLessThan(clearance);
  });
});
