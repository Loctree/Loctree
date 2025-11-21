#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, List, Optional, Sequence, Set, Tuple

LARGE_FILE_THRESHOLD = 1000
COLOR_RED = "\033[31m"
COLOR_RESET = "\033[0m"


def parse_extensions(raw: str) -> Optional[Set[str]]:
    cleaned = {
        segment.strip().lstrip(".").lower()
        for segment in raw.split(",")
        if segment.strip()
    }
    return cleaned or None


@dataclass
class Options:
    extensions: Optional[Set[str]]
    ignore_paths: List[Path]
    use_gitignore: bool
    max_depth: Optional[int]
    color: str
    output: str
    summary: bool
    summary_limit: int


class GitIgnoreChecker:
    def __init__(self, root: Path) -> None:
        self.root = root

    @classmethod
    def build(cls, root: Path) -> Optional["GitIgnoreChecker"]:
        try:
            result = subprocess.run(
                ["git", "-C", str(root), "rev-parse", "--is-inside-work-tree"],
                check=False,
                capture_output=True,
            )
        except OSError:
            return None
        if result.returncode != 0:
            return None
        return cls(root)

    def is_ignored(self, relative_path: Path) -> bool:
        if not relative_path.parts:
            return False
        try:
            result = subprocess.run(
                ["git", "-C", str(self.root), "check-ignore", "-q", str(relative_path)],
                check=False,
            )
        except OSError:
            return False
        return result.returncode == 0


def should_ignore(
    full_path: Path,
    relative_path: Path,
    options: Options,
    git_checker: Optional[GitIgnoreChecker],
) -> bool:
    for ignored in options.ignore_paths:
        try:
            full_path.relative_to(ignored)
            return True
        except ValueError:
            continue
    if options.use_gitignore and git_checker:
        if git_checker.is_ignored(relative_path):
            return True
    return False


def format_label(prefix_flags: Sequence[bool], name: str, is_last: bool) -> str:
    prefix = "".join("│   " if has_next else "    " for has_next in prefix_flags)
    branch = "└── " if is_last else "├── "
    return f"{prefix}{branch}{name}"


def count_file_lines(path: Path) -> Optional[int]:
    try:
        with path.open("r", encoding="utf-8", errors="ignore") as handle:
            return sum(1 for _ in handle)
    except OSError:
        return None


def collect_lines(root: Path, options: Options) -> Tuple[
    List[Tuple[str, Optional[int], str, bool, bool]],
    List[Tuple[str, int]],
    Dict[str, int],
]:
    lines: List[Tuple[str, Optional[int], str, bool, bool]] = []
    large_entries: List[Tuple[str, int]] = []
    stats: Dict[str, int] = {
        "directories": 0,
        "files": 0,
        "filesWithLoc": 0,
        "totalLoc": 0,
    }
    git_checker = GitIgnoreChecker.build(root) if options.use_gitignore else None

    def walk(current: Path, prefix_flags: List[bool], depth: int) -> bool:
        try:
            entries = sorted(
                current.iterdir(),
                key=lambda p: (0 if p.is_dir() else 1, p.name.lower()),
            )
        except OSError:
            return False

        any_included = False
        for index, entry in enumerate(entries):
            if entry.name == ".DS_Store":
                continue
            is_last = index == len(entries) - 1
            label = format_label(prefix_flags, entry.name, is_last)
            full_path = entry.resolve()
            try:
                relative = full_path.relative_to(root)
            except ValueError:
                relative = Path(entry.name)

            if should_ignore(full_path, relative, options, git_checker):
                continue

            loc: Optional[int] = None
            is_dir = entry.is_dir()
            include_current = False

            if is_dir:
                if options.max_depth is None or depth < options.max_depth:
                    child_has = walk(entry, prefix_flags + [not is_last], depth + 1)
                    if child_has:
                        stats["directories"] += 1
                        include_current = True
            else:
                ext = entry.suffix.lstrip(".").lower()
                if options.extensions is None or ext in options.extensions:
                    loc = count_file_lines(full_path)
                    if loc is not None:
                        stats["files"] += 1
                        stats["filesWithLoc"] += 1
                        stats["totalLoc"] += loc
                        is_large = loc >= LARGE_FILE_THRESHOLD
                        if is_large:
                            large_entries.append((str(relative), loc))
                        lines.append(
                            (
                                label,
                                loc,
                                str(relative),
                                False,
                                is_large,
                            )
                        )
                        include_current = True

            if include_current and is_dir:
                lines.append((label, None, str(relative), True, False))

            any_included = any_included or include_current

        return any_included

    walk(root, [], 0)
    return lines, large_entries, stats


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Tree view with LOC counts",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "roots",
        nargs="*",
        default=["."],
        help="One or more folders to inspect (defaults to current directory)",
    )
    parser.add_argument(
        "--ext",
        type=str,
        default="",
        help="Comma-separated extensions to include (e.g. rs,ts,tsx)",
    )
    parser.add_argument(
        "-I",
        "--ignore",
        action="append",
        default=[],
        help="Ignore a folder/file (relative to root or absolute). Repeatable.",
    )
    parser.add_argument(
        "--gitignore",
        "-g",
        action="store_true",
        help="Respect Git ignore rules",
    )
    parser.add_argument(
        "-L",
        "--max-depth",
        type=int,
        default=None,
        help="Limit recursion depth (0 = direct children only)",
    )
    parser.add_argument(
        "--color",
        "-c",
        nargs="?",
        const="always",
        choices=["auto", "always", "never"],
        default="auto",
        help="Colorize large files: auto|always|never (default auto).",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Emit JSON instead of tree output",
    )
    parser.add_argument(
        "--summary",
        nargs="?",
        const="5",
        help="Print totals and top-N large files (default 5)",
    )

    args = parser.parse_args()
    if args.max_depth is not None and args.max_depth < 0:
        raise SystemExit("--max-depth must be non-negative")

    roots = [Path(r).expanduser().resolve() for r in args.roots]
    for root_path in roots:
        if not root_path.is_dir():
            raise SystemExit(f"{root_path} is not a directory")

    extensions = parse_extensions(args.ext) if args.ext else None

    ignore_paths: List[Path] = []
    for raw in args.ignore:
        candidate = Path(raw)
        if not candidate.is_absolute():
            candidate = (root_path / candidate).resolve()
        else:
            candidate = candidate.resolve()
        ignore_paths.append(candidate)

    summary_limit = 5
    if args.summary is not None:
        try:
            summary_limit = int(args.summary)
            if summary_limit <= 0:
                raise ValueError
        except ValueError:
            raise SystemExit("--summary expects a positive integer when provided")

    options = Options(
        extensions=extensions,
        ignore_paths=ignore_paths,
        use_gitignore=args.gitignore,
        max_depth=args.max_depth,
        color=args.color,
        output="json" if args.json else "human",
        summary=args.summary is not None,
        summary_limit=summary_limit,
    )

    results = []
    for idx, root_path in enumerate(roots):
        lines, large_entries, stats = collect_lines(root_path, options)
        sorted_large = sorted(large_entries, key=lambda item: item[1], reverse=True)
        summary = {
            "directories": stats["directories"],
            "files": stats["files"],
            "filesWithLoc": stats["filesWithLoc"],
            "totalLoc": stats["totalLoc"],
            "largeFiles": sorted_large[: options.summary_limit],
        }

        if options.output == "json":
            entries = [
                {
                    "path": rel,
                    "type": "dir" if is_dir else "file",
                    "loc": loc,
                    "isLarge": is_large,
                }
                for (_, loc, rel, is_dir, is_large) in lines
            ]
            payload = {
                "root": str(root_path),
                "options": {
                    "exts": sorted(options.extensions) if options.extensions else None,
                    "ignore": [str(p) for p in options.ignore_paths],
                    "maxDepth": options.max_depth,
                    "useGitignore": options.use_gitignore,
                    "color": options.color,
                    "summary": options.summary_limit if options.summary else False,
                },
                "summary": summary,
                "entries": entries,
            }
            results.append(payload)
            continue

        if idx > 0:
            print("")

        if not lines:
            print(f"{root_path.name or root_path}/ (empty)")
            continue

        max_label_len = max(len(label) for label, *_ in lines)
        root_name = root_path.name or root_path.anchor or str(root_path)
        color_enabled = options.color == "always" or (
            options.color == "auto" and sys.stdout.isatty()
        )

        print(f"{root_name}/")
        for label, loc, _rel, _is_dir, is_large in lines:
            if loc is None:
                print(label)
                continue
            line = f"{label.ljust(max_label_len)}  {loc:6d}"
            if color_enabled and is_large:
                print(f"{COLOR_RED}{line}{COLOR_RESET}")
            else:
                print(line)

        if sorted_large:
            print(f"\nLarge files (>= {LARGE_FILE_THRESHOLD} LOC):")
            for rel, loc in sorted_large:
                summary_line = f"  {rel} ({loc} LOC)"
                if color_enabled:
                    print(f"{COLOR_RED}{summary_line}{COLOR_RESET}")
                else:
                    print(summary_line)

        if options.summary:
            print(
                f"\nSummary: directories: {summary['directories']}, files: {summary['files']}, files with LOC: {summary['filesWithLoc']}, total LOC: {summary['totalLoc']}"
            )
            if not sorted_large:
                print("No files exceed the large-file threshold.")

    if options.output == "json":
        if len(results) == 1:
            print(json.dumps(results[0], indent=2))
        else:
            print(json.dumps(results, indent=2))


if __name__ == "__main__":
    main()
