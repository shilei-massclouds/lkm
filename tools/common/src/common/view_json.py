"""View JSON deserialization shared by render tools."""

from __future__ import annotations

from typing import Any

from common.schemas import VIEW_SCHEMA, VIEW_VERSION
from common.view_types import TimelineItem, TimelineRow, ViewEdge, ViewModel, ViewNode


def view_json_to_view_model(data: dict[str, Any]) -> ViewModel:
    """Deserialize view intermediate JSON into a ViewModel."""

    _require_schema(data, VIEW_SCHEMA, VIEW_VERSION)
    view_name = _string(data, "view")
    metadata = _metadata_from_json(_object(data, "metadata"), view_name)
    return ViewModel(
        name=view_name,
        nodes={
            name: _node_from_json(item)
            for name, item in _object(data, "nodes").items()
        },
        edges=[_edge_from_json(item) for item in _list(data, "edges")],
        rankdir=_string(data, "rankdir"),
        graph_format=_string(data, "graph_format"),
        metadata=metadata,
    )


def _metadata_from_json(data: dict[str, Any], view_name: str) -> dict[str, Any]:
    if view_name != "timeline":
        return data

    metadata = dict(data)
    rows = data.get("timeline_rows", [])
    metadata["timeline_rows"] = tuple(_timeline_row_from_json(row) for row in _as_list(rows, "timeline_rows"))
    phase_parents = data.get("phase_parents", {})
    metadata["phase_parents"] = _as_object(phase_parents, "phase_parents")
    return metadata


def _timeline_row_from_json(item: Any) -> TimelineRow:
    data = _as_object(item, "timeline row")
    subphase = data.get("subphase")
    if subphase is not None and not isinstance(subphase, str):
        raise ValueError("timeline row subphase must be a string or null")
    return TimelineRow(
        id=_string(data, "id"),
        phase=_string(data, "phase"),
        subphase=subphase,
        label=_string(data, "label"),
        detail=_string(data, "detail"),
        items=tuple(_timeline_item_from_json(value) for value in _list(data, "items")),
    )


def _timeline_item_from_json(item: Any) -> TimelineItem:
    data = _as_object(item, "timeline item")
    return TimelineItem(
        object_name=_string(data, "object_name"),
        detail=_string(data, "detail"),
        kind=_string(data, "kind"),
    )


def _node_from_json(item: Any) -> ViewNode:
    data = _as_object(item, "node")
    return ViewNode(
        id=_string(data, "id"),
        label=_string(data, "label"),
        kind=_string(data, "kind"),
    )


def _edge_from_json(item: Any) -> ViewEdge:
    data = _as_object(item, "edge")
    return ViewEdge(
        source=_string(data, "source"),
        target=_string(data, "target"),
        kind=_string(data, "kind"),
        label=_string(data, "label"),
    )


def _require_schema(data: dict[str, Any], schema: str, version: int) -> None:
    if data.get("schema") != schema:
        raise ValueError(f"expected schema {schema!r}")
    if data.get("version") != version:
        raise ValueError(f"expected version {version}")


def _object(data: dict[str, Any], key: str) -> dict[str, Any]:
    return _as_object(data.get(key), key)


def _list(data: dict[str, Any], key: str) -> list[Any]:
    return _as_list(data.get(key), key)


def _string(data: dict[str, Any], key: str) -> str:
    return _as_string(data.get(key), key)


def _as_object(value: Any, name: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ValueError(f"{name} must be an object")
    return value


def _as_list(value: Any, name: str) -> list[Any]:
    if not isinstance(value, list):
        raise ValueError(f"{name} must be a list")
    return value


def _as_string(value: Any, name: str) -> str:
    if not isinstance(value, str):
        raise ValueError(f"{name} must be a string")
    return value
