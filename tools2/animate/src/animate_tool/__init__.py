"""Build deterministic, self-contained tools2 Signal animation documents."""

from .builder import build_animation
from .html import render_html, write_html

__all__ = ["build_animation", "render_html", "write_html"]
