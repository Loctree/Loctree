# Dead Exports Testing Methodology

## Cel
Weryfikacja jakości wykrywania dead exports w loctree poprzez testowanie na real-world repozytoriach.

## Krok 1: Baseline scan
```bash
cd /Users/maciejgad/hosted/loctree/loctree_rs
./target/release/loctree <REPO_PATH> --dead 2>/dev/null > /tmp/<repo>_dead_baseline.txt
head -1 /tmp/<repo>_dead_baseline.txt  # pokazuje liczbę dead exports
```

## Krok 2: Sampling (20-50 przypadków)
```bash
# Losowa próbka z dead exports
shuf /tmp/<repo>_dead_baseline.txt | head -50 > /tmp/<repo>_sample.txt
```

## Krok 3: Weryfikacja manualna

Dla każdego przypadku z sample:
1. **Sprawdź czy symbol jest faktycznie importowany:**
   ```bash
   cd <REPO_PATH>
   rg "import.*<symbol_name>" --type ts --type js -l
   rg "from.*<symbol_name>" --type ts --type js -l
   ```

2. **Jeśli import istnieje - to FALSE POSITIVE. Zapisz:**
   - Ścieżkę importu (np. `$lib/`, `@scope/`, relative)
   - Framework (SvelteKit, Next.js, etc.)
   - Wzorzec (alias, barrel, dynamic import)

3. **Jeśli import nie istnieje - sprawdź czy to entry point:**
   - Framework routing (`+page.ts`, `page.tsx`)
   - Config files (`*.config.ts`)
   - Test fixtures
   - Type declarations (`.d.ts`)

## Krok 4: Kategoryzacja false positives

| Kategoria | Przykład | Rozwiązanie |
|-----------|----------|-------------|
| Alias nierozpoznany | `$lib/foo` | Dodaj do parsera aliasów |
| Plik nieobsługiwany | `.svelte`, `.vue` | Dodaj parser ekstensji |
| Entry point frameworka | `+page.ts` | Dodaj do skip patterns |
| Config file | `vite.config.ts` | Dodaj do skip patterns |
| .d.ts | `types.d.ts` | Już obsłużone |
| Barrel re-export | `index.ts` | Sprawdź star re-exports |

## Krok 5: Raportowanie

Format raportu:
```
Repo: <nazwa>
Baseline: <liczba> dead exports
Sample size: <n>
False positives: <liczba> (<procent>%)
Kategorie:
- Alias: <liczba>
- Extension: <liczba>
- Entry point: <liczba>
- Inne: <lista>
```

## Krok 6: Po naprawach

Po wprowadzeniu zmian w loctree:
```bash
cargo build --release
./target/release/loctree <REPO_PATH> --dead 2>/dev/null | head -1
```

Porównaj z baseline - poprawa powinna być widoczna.

## Repozytoria testowe

Już przetestowane:
- GitButler: 2883 → 1429 (po dodaniu SvelteKit + fallback)
- SillyTavern: 68 → 48

Sugerowane do dalszych testów:
- Duże monorepo TypeScript
- Next.js app
- Vue.js project
- Python project z wieloma modułami

## Uwagi

1. **Nie dodawaj wykluczeń pojedynczo** - szukaj wzorców
2. **Sprawdzaj czy to prawdziwy dead code** - niektóre przypadki to faktycznie martwy kod
3. **Dokumentuj znalezione wzorce** - pomaga w przyszłych naprawach
