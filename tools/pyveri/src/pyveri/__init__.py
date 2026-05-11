"""Static derivation verifier for LKM object model specs."""

from .parser import ParseError, parse_file, parse_text, strip_comments

__all__ = ["ParseError", "parse_file", "parse_text", "strip_comments"]
