export function shouldRenderSignalArrow(source: Element, target: Element): boolean {
  return source === target || !source.contains(target);
}
