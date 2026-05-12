"""View JSON serialization for the view stage."""

from __future__ import annotations

from dataclasses import is_dataclass
from typing import Any

from common import VIEW_SCHEMA, VIEW_VERSION
from pyveri.view import ViewEdge, ViewModel, ViewNode


def view_to_json(view: ViewModel, input_data: dict[str, Any]) -> dict[str, Any]:
    """Serialize a pyveri view model to the view intermediate format."""

    return {
        "schema": VIEW_SCHEMA,
        "version": VIEW_VERSION,
        "source": input_data.get("source"),
        "input": {
            "schema": input_data.get("schema"),
            "version": input_data.get("version"),
        },
        "view": view.name,
        "rankdir": view.rankdir,
        "graph_format": view.graph_format,
        "nodes": {
            name: _node_to_json(node)
            for name, node in sorted(view.nodes.items())
        },
        "edges": [_edge_to_json(edge) for edge in view.edges],
        "metadata": _to_jsonable(view.metadata),
    }


def _node_to_json(node: ViewNode) -> dict[str, str]:
    return {
        "id": node.id,
        "label": node.label,
        "kind": node.kind,
    }


def _edge_to_json(edge: ViewEdge) -> dict[str, str]:
    return {
        "source": edge.source,
        "target": edge.target,
        "kind": edge.kind,
        "label": edge.label,
    }


def _to_jsonable(value: Any) -> Any:
    if is_dataclass(value):
        return {
            key: _to_jsonable(item)
            for key, item in value.__dict__.items()
        }
    if isinstance(value, dict):
        return {str(key): _to_jsonable(item) for key, item in value.items()}
    if isinstance(value, (list, tuple)):
        return [_to_jsonable(item) for item in value]
    return value
