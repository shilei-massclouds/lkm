class ImmediateAnimation {
  currentTime = 0;
  effect: AnimationEffect | null = null;
  playState: AnimationPlayState = 'finished';
  private finish: Animation['onfinish'] = null;

  cancel() {}

  get onfinish() {
    return this.finish;
  }

  set onfinish(callback: Animation['onfinish']) {
    this.finish = callback;
    if (callback) queueMicrotask(() => callback.call(
      this as unknown as Animation,
      new Event('finish') as AnimationPlaybackEvent
    ));
  }
}

if (!Element.prototype.getAnimations) {
  Element.prototype.getAnimations = () => [];
}
if (!Element.prototype.animate) {
  Element.prototype.animate = () => new ImmediateAnimation() as unknown as Animation;
}
