#!/usr/bin/env python3
from __future__ import annotations

import argparse
from pathlib import Path
from typing import List, Optional, Set, Tuple


def collect_lines(root: Path, exts: Optional[Set[str]]) -> List[Tuple[str, Optional[int]]]:
    """
    Zwraca listę (label, loc):
      - label = gotowy string z gałązką drzewa + nazwą pliku/katalogu
      - loc   = liczba linii (dla plików, jeśli pasują rozszerzeniem), albo None
    """

    lines: List[Tuple[str, Optional[int]]] = []

    def sort_key(p: Path):
        # katalogi najpierw, potem pliki; wszystko po nazwie case-insensitive
        return (0 if p.is_dir() else 1, p.name.lower())

    def walk(dir_path: Path, prefix_parts: List[bool]) -> None:
        entries = sorted(list(dir_path.iterdir()), key=sort_key)
        for idx, entry in enumerate(entries):
            is_last = idx == len(entries) - 1

            # prefix z pionowymi kreskami / spacjami
            prefix = ""
            for has_more in prefix_parts:
                prefix += "│   " if has_more else "    "

            branch = "└── " if is_last else "├── "
            label = prefix + branch + entry.name

            loc: Optional[int] = None
            if entry.is_file():
                if exts is None or entry.suffix.lstrip(".") in exts:
                    try:
                        with entry.open("r", encoding="utf-8", errors="ignore") as f:
                            loc = sum(1 for _ in f)
                    except OSError:
                        loc = None

            lines.append((label, loc))

            if entry.is_dir():
                # jeśli to nie ostatni element, to w kolejnych poziomach rysujemy │
                walk(entry, prefix_parts + [not is_last])

    walk(root, [])
    return lines


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Tree z kolumną LOC dla katalogu (pół‑eza, pół‑psycho)."
    )
    parser.add_argument(
        "root",
        type=str,
        help="Katalog startowy (np. src-tauri/src)",
    )
    parser.add_argument(
        "--ext",
        type=str,
        default="",
        help="Lista rozszerzeń, po przecinku (np. 'rs,ts,tsx'). Puste = wszystkie pliki.",
    )
    args = parser.parse_args()

    root_path = Path(args.root).resolve()
    if not root_path.is_dir():
        raise SystemExit(f"{root_path} nie jest katalogiem")

    exts: Optional[Set[str]]
    if args.ext.strip():
        exts = {e.strip().lstrip(".") for e in args.ext.split(",") if e.strip()}
    else:
        exts = None

    lines = collect_lines(root_path, exts)
    if not lines:
        return

    max_label_len = max(len(label) for label, _ in lines)

    # nagłówek z nazwą root
    print(f"{root_path.name}/")

    for label, loc in lines:
        if loc is None:
            print(label)
        else:
            padding = " " * (max_label_len - len(label) + 2)
            print(f"{label}{padding}{loc:6d}")


if __name__ == "__main__":
    main()