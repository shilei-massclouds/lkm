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
  direction: 'left' | 'right' | 'up' | 'down' | 'self';
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
    const loopHeight = source.height * 0.62 + source.width * 0.18;
    const loopY = y(source.top) - loopHeight;
    const apexY = centerY * 0.25 + loopY * 0.75;
    const labelGap = Math.max(source.height * 0.12, source.width * 0.04);
    return {
      path: `M ${startX} ${centerY} C ${startX + controlReach} ${loopY}, ${endX - controlReach} ${loopY}, ${endX} ${centerY}`,
      labelX: (startX + endX) / 2,
      labelY: apexY - labelGap,
      self: true,
      direction: 'self'
    };
  }
  const sourceCenterX = source.left + source.width / 2;
  const sourceCenterY = source.top + source.height / 2;
  const targetCenterX = target.left + target.width / 2;
  const targetCenterY = target.top + target.height / 2;
  const deltaX = targetCenterX - sourceCenterX;
  const deltaY = targetCenterY - sourceCenterY;
  if (Math.abs(deltaX) >= Math.abs(deltaY)) {
    const towardRight = deltaX >= 0;
    const startX = x(towardRight ? source.right : source.left);
    const startY = y(sourceCenterY);
    const endX = x(towardRight ? target.left : target.right);
    const endY = y(targetCenterY);
    return {
      path: `M ${startX} ${startY} L ${endX} ${endY}`,
      labelX: (startX + endX) / 2,
      labelY: (startY + endY) / 2,
      self: false,
      direction: towardRight ? 'right' : 'left'
    };
  }
  const towardDown = deltaY >= 0;
  const startX = x(sourceCenterX);
  const startY = y(towardDown ? source.bottom : source.top);
  const endX = x(targetCenterX);
  const endY = y(towardDown ? target.top : target.bottom);
  return {
    path: `M ${startX} ${startY} L ${endX} ${endY}`,
    labelX: (startX + endX) / 2,
    labelY: (startY + endY) / 2,
    self: false,
    direction: towardDown ? 'down' : 'up'
  };
}
