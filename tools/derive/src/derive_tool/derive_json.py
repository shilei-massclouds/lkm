"""Derivation JSON serialization for the derive stage."""

from __future__ import annotations

from typing import Any

from common import DERIVE_SCHEMA, DERIVE_VERSION
from pyveri.ast import SourceSpan
from pyveri.derive import (
    DerivationRecord,
    DerivationResult,
    DerivationStatus,
    EventTransition,
)


def derivation_to_json(
    result: DerivationResult, model_data: dict[str, Any]
) -> dict[str, Any]:
    """Serialize a derivation result to the derive intermediate format."""

    counts = _record_counts(result.records)
    return {
        "schema": DERIVE_SCHEMA,
        "version": DERIVE_VERSION,
        "source": model_data.get("source"),
        "input": {
            "schema": model_data.get("schema"),
            "version": model_data.get("version"),
        },
        "target": {
            "event": result.target,
            "object": result.target_object,
            "name": result.target_event,
            "state": result.target_state,
            "reached": result.target_reached,
        },
        "summary": {
            "ok": result.ok,
            "target_reached": result.target_reached,
            "transitions": len(result.transitions),
            "proved": counts[DerivationStatus.PROVED.value],
            "assumed": counts[DerivationStatus.ASSUMED.value],
            "obligation": counts[DerivationStatus.OBLIGATION.value],
            "deferred": counts[DerivationStatus.DEFERRED.value],
            "blocked": counts[DerivationStatus.BLOCKED.value],
            "contradiction": counts[DerivationStatus.CONTRADICTION.value],
        },
        "states": dict(sorted(result.states.items())),
        "records": [_record_to_json(record) for record in result.records],
        "transitions": [
            _transition_to_json(transition) for transition in result.transitions
        ],
    }


def _record_counts(records: tuple[DerivationRecord, ...]) -> dict[str, int]:
    counts = {status.value: 0 for status in DerivationStatus}
    for record in records:
        counts[record.status.value] += 1
    return counts


def _record_to_json(record: DerivationRecord) -> dict[str, Any]:
    return {
        "status": record.status.value,
        "message": record.message,
        "span": _optional_span_to_json(record.span),
        "object": record.object_name,
        "event": record.event_name,
        "state": record.state_name,
        "expression": record.expression,
    }


def _transition_to_json(transition: EventTransition) -> dict[str, str]:
    return {
        "object": transition.object_name,
        "event": transition.event_name,
        "source_state": transition.source_state,
        "target_state": transition.target_state,
        "label": transition.label,
    }


def _optional_span_to_json(span: SourceSpan | None) -> dict[str, int] | None:
    if span is None:
        return None
    return {
        "start_line": span.start_line,
        "end_line": span.end_line,
    }
