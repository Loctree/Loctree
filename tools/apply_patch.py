#!/usr/bin/env python3
from __future__ import annotations

import os
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import List, Sequence

BEGIN = "*** Begin Patch"
END = "*** End Patch"
ADD = "*** Add File:"
UPDATE = "*** Update File:"
DELETE = "*** Delete File:"
MOVE = "*** Move to:"
HUNK_HEADER_RE = re.compile(r"@@ -(?P<orig>\d+)(?:,\d+)? \+(?P<new>\d+)(?:,\d+)? @@")


class PatchError(Exception):
    """Raised when the patch format or application fails."""


@dataclass
class Hunk:
    header: str
    lines: List[str]

    def __post_init__(self) -> None:
        match = HUNK_HEADER_RE.match(self.header.strip())
        self.original_start = int(match.group("orig")) if match else None


@dataclass
class Operation:
    kind: str
    path: str
    new_path: str | None = None
    payload: object | None = None


def read_patch_text(argv: List[str]) -> str:
    if len(argv) > 2:
        raise PatchError("apply_patch accepts at most one argument")
    if len(argv) == 2:
        candidate = argv[1]
        if candidate == "-":
            data = sys.stdin.read()
            if not data.strip():
                raise PatchError("No patch data provided via stdin")
            return data
        candidate_path = Path(candidate)
        if candidate_path.exists():
            data = candidate_path.read_text(encoding="utf-8")
            if not data.strip():
                raise PatchError(f"Patch file '{candidate}' is empty")
            return data
        if BEGIN in candidate:
            return candidate
        # If the argument does not look like a patch yet refers to a missing file,
        # surface an explicit error for easier debugging.
        raise PatchError(f"Patch file '{candidate}' does not exist")
    data = sys.stdin.read()
    if not data.strip():
        raise PatchError("No patch data provided")
    return data


def normalize_lines(text: str) -> List[str]:
    text = text.replace("\r\n", "\n").replace("\r", "\n")
    lines = text.split("\n")
    # split() leaves a trailing empty element when the input ends with a newline,
    # but patch parsing is easier without it.
    if lines and lines[-1] == "":
        lines.pop()
    if lines and lines[0].startswith("\ufeff"):
        lines[0] = lines[0].lstrip("\ufeff")
    return lines


def parse_patch(text: str) -> List[Operation]:
    lines = normalize_lines(text)
    idx = 0
    while idx < len(lines) and not lines[idx].strip():
        idx += 1
    if idx >= len(lines) or lines[idx].strip() != BEGIN:
        raise PatchError("Patch must start with '*** Begin Patch'")
    idx += 1

    operations: List[Operation] = []
    while idx < len(lines):
        if not lines[idx].strip():
            idx += 1
            continue
        if lines[idx].strip() == END:
            return operations
        line = lines[idx]
        if line.startswith(ADD):
            path = line[len(ADD):].strip()
            idx += 1
            payload: List[str] = []
            while idx < len(lines):
                row = lines[idx]
                if row.startswith("*** "):
                    break
                if not row.startswith("+"):
                    raise PatchError("Added file lines must start with '+'")
                payload.append(row[1:])
                idx += 1
            if not path:
                raise PatchError("Missing path for add operation")
            operations.append(Operation("add", path, payload=payload))
            continue
        if line.startswith(DELETE):
            path = line[len(DELETE):].strip()
            if not path:
                raise PatchError("Missing path for delete operation")
            operations.append(Operation("delete", path))
            idx += 1
            continue
        if line.startswith(UPDATE):
            path = line[len(UPDATE):].strip()
            if not path:
                raise PatchError("Missing path for update operation")
            idx += 1
            new_path: str | None = None
            if idx < len(lines) and lines[idx].startswith(MOVE):
                new_path = lines[idx][len(MOVE):].strip()
                idx += 1
            hunks: List[Hunk] = []
            current_header: str | None = None
            current_lines: List[str] = []
            chunk_started = False
            while idx < len(lines):
                row = lines[idx]
                if row.startswith("*** "):
                    break
                chunk_started = True
                if row == "*** End of File":
                    idx += 1
                    continue
                if row.startswith("@@"):
                    if current_header is not None:
                        hunks.append(Hunk(current_header, current_lines))
                    current_header = row
                    current_lines = []
                else:
                    if current_header is None:
                        current_header = "@@"
                    current_lines.append(row)
                idx += 1
            if current_header is not None:
                hunks.append(Hunk(current_header, current_lines))
            if not hunks:
                if chunk_started:
                    hunks.append(Hunk("@@", []))
                else:
                    raise PatchError(f"Update '{path}' missing hunk data")
            operations.append(Operation("update", path, new_path=new_path, payload=hunks))
            continue
        raise PatchError(f"Unsupported patch directive: {line}")
    raise PatchError("Patch missing '*** End Patch'")


def ensure_parent(path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)


def write_file(path: Path, lines: Sequence[str]) -> None:
    ensure_parent(path)
    text = "\n".join(lines)
    if lines:
        text += "\n"
    path.write_text(text, encoding="utf-8")


def delete_file(path: Path) -> None:
    if not path.exists():
        raise PatchError(f"Cannot delete missing file: {path}")
    if path.is_file():
        path.unlink()
    else:
        raise PatchError(f"Cannot delete non-file path: {path}")


def decode_hunk_lines(lines: Sequence[str]) -> tuple[List[str], List[str]]:
    original: List[str] = []
    replacement: List[str] = []
    for row in lines:
        if not row:
            raise PatchError("Malformed hunk line")
        if row.startswith("\\ "):
            # "\\ No newline at end of file" marker – ignore.
            continue
        prefix = row[0]
        content = row[1:]
        if prefix == " ":
            original.append(content)
            replacement.append(content)
        elif prefix == "-":
            original.append(content)
        elif prefix == "+":
            replacement.append(content)
        else:
            raise PatchError(f"Unsupported hunk prefix '{prefix}'")
    return original, replacement


def apply_update(path: Path, hunks: Sequence[Hunk]) -> None:
    if not path.exists():
        raise PatchError(f"Cannot update missing file: {path}")
    text = path.read_text(encoding="utf-8")
    trailing_newline = text.endswith("\n")
    lines = text.splitlines()
    line_delta = 0

    for hunk in hunks:
        original, replacement = decode_hunk_lines(hunk.lines)
        hint = None
        if hunk.original_start is not None:
            hint = max(hunk.original_start - 1 + line_delta, 0)
        idx = find_sequence(lines, original, hint)
        lines[idx:idx + len(original)] = replacement
        if hunk.original_start is not None:
            line_delta += len(replacement) - len(original)

    new_text = "\n".join(lines)
    if trailing_newline or not lines:
        new_text += "\n"
    path.write_text(new_text, encoding="utf-8")


def find_sequence(lines: Sequence[str], seq: Sequence[str], start_hint: int | None = None) -> int:
    if not seq:
        if start_hint is not None:
            return max(0, min(start_hint, len(lines)))
        return len(lines)
    max_start = len(lines) - len(seq)
    if max_start < 0:
        raise PatchError("Patch hunk longer than target file")

    positions: List[int] = []
    if start_hint is not None:
        start = max(0, min(start_hint, max_start))
        positions.extend(range(start, max_start + 1))
        if start > 0:
            positions.extend(range(0, start))
    else:
        positions = list(range(0, max_start + 1))

    for pos in positions:
        if lines[pos:pos + len(seq)] == list(seq):
            return pos
    raise PatchError("Failed to match patch hunk context")


def apply_operations(ops: Sequence[Operation]) -> None:
    for op in ops:
        target = Path(op.path)
        if op.kind == "add":
            write_file(target, op.payload)  # type: ignore[arg-type]
        elif op.kind == "delete":
            delete_file(target)
        elif op.kind == "update":
            apply_update(target, op.payload)  # type: ignore[arg-type]
            if op.new_path:
                new_path = Path(op.new_path)
                ensure_parent(new_path)
                target.rename(new_path)
        else:
            raise PatchError(f"Unsupported operation '{op.kind}'")


def main(argv: List[str]) -> None:
    text = read_patch_text(argv)
    operations = parse_patch(text)
    apply_operations(operations)


if __name__ == "__main__":
    try:
        main(sys.argv)
    except PatchError as exc:
        print(f"apply_patch: {exc}", file=sys.stderr)
        sys.exit(1)
