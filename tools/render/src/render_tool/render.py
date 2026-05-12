"""Renderer dispatch for view models."""

from __future__ import annotations

from pyveri.view import ViewModel, render_dot, render_svg, render_text


def render_view(view: ViewModel, fmt: str) -> str:
    """Render a view model with the requested output format."""

    if fmt == "text":
        return render_text(view)
    if fmt == "dot":
        return render_dot(view)
    if fmt == "svg":
        return render_svg(view)
    raise ValueError(f"unknown render format: {fmt}")
