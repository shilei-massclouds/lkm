"""Render and atomically publish the self-contained animation HTML."""

from __future__ import annotations

import json
from html import escape
import os
from pathlib import Path
import re
import tempfile
from typing import Any


_FRONTEND_DIST = Path(__file__).resolve().parents[2] / "frontend" / "dist"


def _script_json(value: Any) -> str:
    return (
        json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
        .replace("&", "\\u0026")
        .replace("<", "\\u003c")
        .replace(">", "\\u003e")
        .replace("\u2028", "\\u2028")
        .replace("\u2029", "\\u2029")
    )


def render_html(animation: dict[str, Any], *, dist: Path | None = None) -> str:
    bundle_root = dist or _FRONTEND_DIST
    css = (bundle_root / "player.css").read_text(encoding="utf-8")
    javascript = (bundle_root / "player.js").read_text(encoding="utf-8")
    title = animation.get("trace", {}).get("root_request", {}).get("target", "Signal trace")
    css = re.sub(r"</style", r"<\\/style", css, flags=re.IGNORECASE)
    javascript = re.sub(r"</script", r"<\\/script", javascript, flags=re.IGNORECASE)
    return f"""<!doctype html>
<html lang=\"en\">
<head>
  <meta charset=\"utf-8\">
  <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">
  <title>{escape(str(title))} · Signal trace</title>
  <style>{css}</style>
</head>
<body>
  <main id=\"lkm-signal-player\"></main>
  <script id=\"lkm-signal-animation\" type=\"application/json\">{_script_json(animation)}</script>
  <script>{javascript}</script>
</body>
</html>
"""


def write_html(path: str | Path, animation: dict[str, Any], *, dist: Path | None = None) -> None:
    target = Path(path)
    target.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary = tempfile.mkstemp(prefix=f".{target.name}.", dir=target.parent)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8", newline="\n") as stream:
            stream.write(render_html(animation, dist=dist))
        os.replace(temporary, target)
    except BaseException:
        try:
            os.unlink(temporary)
        except FileNotFoundError:
            pass
        raise


__all__ = ["render_html", "write_html"]
