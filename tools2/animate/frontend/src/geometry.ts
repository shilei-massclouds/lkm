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
  scrollTop = 0,
  self = false
): ArrowGeometry {
  const x = (value: number) => value - stage.left + scrollLeft;
  const y = (value: number) => value - stage.top + scrollTop;
  if (self) {
    const startX = x(source.right);
    const endX = x(source.left);
    const centerY = y(source.top + source.height / 2);
    const controlReach = source.width * 0.36;
    const loopDepth = source.height * 0.62 + source.width * 0.18;
    const loopY = y(source.bottom) + loopDepth;
    return {
      path: `M ${startX} ${centerY} C ${startX + controlReach} ${loopY}, ${endX - controlReach} ${loopY}, ${endX} ${centerY}`,
      labelX: (startX + endX) / 2,
      labelY: loopY + source.height * 0.05,
      self: true
    };
  }
  const startX = x(source.right);
  const startY = y(source.top + source.height / 2);
  const endX = x(target.left);
  const endY = y(target.top + target.height / 2);
  const distance = Math.abs(endX - startX);
  const meanWidth = (source.width + target.width) / 2;
  const bend = Math.max(distance * 0.3, meanWidth * 0.35);
  const direction = endX >= startX ? 1 : -1;
  return {
    path: `M ${startX} ${startY} C ${startX + bend * direction} ${startY}, ${endX - bend * direction} ${endY}, ${endX} ${endY}`,
    labelX: (startX + endX) / 2,
    labelY: (startY + endY) / 2 - Math.max(source.height, target.height) * 0.12,
    self: false
  };
}
