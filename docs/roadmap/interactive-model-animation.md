# Interactive tools2 Signal animation

本页记录当前离线动画边界；规范性语义以 Charter/Model/Coding 为准。

## Current pipeline

```text
tools2 v10 model.json ----+
                          +--> animate --> animation v4 self-contained HTML
tools2 v10 view.json -----+
```

animate 只重放 view 已确定的 event sequence，不重新求值 guard、choice、Signal delivery 或 transition。
前端只播放预生成 moments/frames，不在浏览器 derive。

animation v4 显示：

- Signal request、drives/root feedback、emits/yields target settle 与 terminal outcome；
- YieldToken creation 对应的 yield moment、精确一次 resume moment，以及 token/lane/resume coordinate；
- Transition before/after state、Action 无伪造 lifecycle delta、handler Model description；
- stable first-seen hierarchy、确定 sibling order、前进/后退的完全可逆 frame；
- boundary inventory/occurrence/obligation 的非因果证据视图。

协议为 `schema=lkm.spec.signal-animation`、`producer=tools2`、version 4。生成器只接受 tools2 v10
model/view，并严格拒绝旧协议；老自包含 HTML 可由其内嵌旧播放器独立打开，但不提供旧格式生成开关。

## Validation

- Python animation protocol tests cover schema/version/producer、event/snapshot ordering、all outcomes、nested
  delivery、yield/resume exact-once and deterministic regeneration.
- Svelte/TypeScript tests cover hierarchy/layout, arrows, response effects, token control text, navigation and
  reduced motion.
- Browser tests load the self-contained file through `file://`, reject network requests and traverse every moment.
- Canonical gates are `make -C tools2 test-frontend`, `bundle-check`, `test-browser`, and `test-all`.
