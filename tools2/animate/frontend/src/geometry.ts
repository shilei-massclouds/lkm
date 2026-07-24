export interface RectLike {
  left: number;
  right: number;
  top: number;
  bottom: number;
  width: number;
  height: number;
}

export interface ArrowGeometry {
  path: string;
  labelX: number;
  labelY: number;
  self: boolean;
}

export function signalGeometry(
  source: RectLike,
  target: RectLike,
  stage: RectLike,
  scrollLeft = 0,
  scrollTop = 0
): ArrowGeometry {
  const x = (value: number) => value - stage.left + scrollLeft;
  const y = (value: number) => value - stage.top + scrollTop;
  if (source === target) {
    const startX = x(source.right);
    const endX = x(source.left);
    const centerY = y(source.top + source.height / 2);
    const loopY = y(source.bottom) + Math.max(58, source.height * 0.62);
    return {
      path: `M ${startX} ${centerY} C ${startX + 72} ${loopY}, ${endX - 72} ${loopY}, ${endX} ${centerY}`,
      labelX: (startX + endX) / 2,
      labelY: loopY + 2,
      self: true
    };
  }
  const startX = x(source.right);
  const startY = y(source.top + source.height / 2);
  const endX = x(target.left);
  const endY = y(target.top + target.height / 2);
  const bend = Math.max(56, Math.abs(endX - startX) * 0.42);
  const direction = endX >= startX ? 1 : -1;
  return {
    path: `M ${startX} ${startY} C ${startX + bend * direction} ${startY}, ${endX - bend * direction} ${endY}, ${endX} ${endY}`,
    labelX: (startX + endX) / 2,
    labelY: (startY + endY) / 2 - 12,
    self: false
  };
}
