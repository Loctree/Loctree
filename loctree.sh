#!/usr/bin/env bash
root="src-tauri/src"

(
  cd "$root" || exit 1

  find . -print | sort | awk '
    {
      gsub(/^\.\//, "", $0);           # usuwamy "./"
      path = $0;
      if (path == "") next;

      n = split(path, parts, "/");
      depth = n - 1;
      name = parts[n];

      # zbuduj wcięcie
      indent = "";
      for (i = 0; i < depth; i++) {
        indent = indent "  ";
      }

      # katalog?
      cmd = sprintf("[ -d \"%s\" ]", path);
      if (system(cmd) == 0) {
        printf "%s%s/\n", indent, name;
      } else {
        # plik: policz LOC
        cmd = sprintf("wc -l < \"%s\"", path);
        cmd | getline loc;
        close(cmd);
        gsub(/^[ \t]+|[ \t]+$/, "", loc); # trim
        printf "%s%6d  %s\n", indent, loc, name;
      }
    }
  '
)