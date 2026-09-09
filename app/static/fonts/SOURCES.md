# Vendored fonts — sources and hashes

Fetched 2026-09-03 from the Google Fonts CSS2 endpoint (`fonts.googleapis.com/css2?family=Instrument+Sans:wght@400;500;600;700&family=JetBrains+Mono:wght@400;500;600&display=swap`), requested with a
modern browser `User-Agent` so the response serves woff2. Each family/weight below is the `latin`
subset `@font-face` block's `src` URL (the endpoint also serves `latin-ext`, `cyrillic`, `cyrillic-ext`,
`greek` and `vietnamese` subsets for these two families; none are vendored). Both families are
variable fonts: the CSS2 endpoint points every requested static weight of a family at the same
binary, distinguished only by the `font-weight` each `@font-face` block declares — so the four
Instrument Sans files are byte-identical to each other, and likewise the three JetBrains Mono files.
That is upstream Google Fonts behaviour, not a vendoring mistake; the hashes below confirm it.

| File | Family | Weight | Source URL | SHA-256 |
| --- | --- | --- | --- | --- |
| instrument-sans-400.woff2 | Instrument Sans | 400 | https://fonts.gstatic.com/s/instrumentsans/v4/pxiTypc9vsFDm051Uf6KVwgkfoSxQ0GsQv8ToedPibnr0SZe1Q.woff2 | 2ee17598a98d8a59e4df8152d015bec9ab8e4d5672cc0ab42bef806b568e3971 |
| instrument-sans-500.woff2 | Instrument Sans | 500 | https://fonts.gstatic.com/s/instrumentsans/v4/pxiTypc9vsFDm051Uf6KVwgkfoSxQ0GsQv8ToedPibnr0SZe1Q.woff2 | 2ee17598a98d8a59e4df8152d015bec9ab8e4d5672cc0ab42bef806b568e3971 |
| instrument-sans-600.woff2 | Instrument Sans | 600 | https://fonts.gstatic.com/s/instrumentsans/v4/pxiTypc9vsFDm051Uf6KVwgkfoSxQ0GsQv8ToedPibnr0SZe1Q.woff2 | 2ee17598a98d8a59e4df8152d015bec9ab8e4d5672cc0ab42bef806b568e3971 |
| instrument-sans-700.woff2 | Instrument Sans | 700 | https://fonts.gstatic.com/s/instrumentsans/v4/pxiTypc9vsFDm051Uf6KVwgkfoSxQ0GsQv8ToedPibnr0SZe1Q.woff2 | 2ee17598a98d8a59e4df8152d015bec9ab8e4d5672cc0ab42bef806b568e3971 |
| jetbrains-mono-400.woff2 | JetBrains Mono | 400 | https://fonts.gstatic.com/s/jetbrainsmono/v24/tDbv2o-flEEny0FZhsfKu5WU4zr3E_BX0PnT8RD8yKwBNntkaToggR7BYRbKPxDcwg.woff2 | 83c005d49d8a6a50474c73a5a36ac0468076e9c4a29da7bdb14995d80560a5be |
| jetbrains-mono-500.woff2 | JetBrains Mono | 500 | https://fonts.gstatic.com/s/jetbrainsmono/v24/tDbv2o-flEEny0FZhsfKu5WU4zr3E_BX0PnT8RD8yKwBNntkaToggR7BYRbKPxDcwg.woff2 | 83c005d49d8a6a50474c73a5a36ac0468076e9c4a29da7bdb14995d80560a5be |
| jetbrains-mono-600.woff2 | JetBrains Mono | 600 | https://fonts.gstatic.com/s/jetbrainsmono/v24/tDbv2o-flEEny0FZhsfKu5WU4zr3E_BX0PnT8RD8yKwBNntkaToggR7BYRbKPxDcwg.woff2 | 83c005d49d8a6a50474c73a5a36ac0468076e9c4a29da7bdb14995d80560a5be |

Hashes taken with `Get-FileHash -Algorithm SHA256`.

## Licences

Both families are SIL Open Font License 1.1.

- `OFL-instrument-sans.txt` — fetched from
  `https://raw.githubusercontent.com/Instrument/instrument-sans/master/OFL.txt`
  (the repo's default branch is `master`, not `main`; `main` 404s).
- `OFL-jetbrains-mono.txt` — fetched from
  `https://raw.githubusercontent.com/JetBrains/JetBrainsMono/master/OFL.txt`.
