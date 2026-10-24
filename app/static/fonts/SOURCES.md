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

## Onboarding faces (wave B, F11p0, fetched 2026-10-24)

Quinn's ruling (2026-10-24): the Knowlu wordmark is set in PT Serif Caption, and every other piece of
text in setup and onboarding is Zilla Slab. The console keeps Instrument Sans and JetBrains Mono for
now; whether it switches too is a separate decision (Q-B24).

**Zilla Slab** was fetched like the faces above, from the CSS2 endpoint
(`fonts.googleapis.com/css2?family=Zilla+Slab:wght@400;600;700&display=swap`, modern `User-Agent`),
latin subset only. Zilla Slab is a static family, so each weight is its own binary. Its licence names
no Reserved Font Name, so Google's latin subset may be shipped under its own name.

**PT Serif Caption** is the unmodified upstream file. It is not a subset, because its licence reserves the
names "PT Sans", "PT Serif" and "ParaType", and under OFL clause 3 a subset is a Modified Version that
could not keep the name. The file comes from `google/fonts`, pinned to the last commit that touched it, and
embeds the OFL 1.1 notice in its name table. Only the wordmark uses it.

| File | Family | Weight | Source URL | SHA-256 |
| --- | --- | --- | --- | --- |
| zilla-slab-400.woff2 | Zilla Slab | 400 | https://fonts.gstatic.com/s/zillaslab/v12/dFa6ZfeM_74wlPZtksIFajo6_Q.woff2 | ab2607313bc04e82a53aca235853a0b83463e99b3ae88b17cc963e371f8fe91c |
| zilla-slab-600.woff2 | Zilla Slab | 600 | https://fonts.gstatic.com/s/zillaslab/v12/dFa5ZfeM_74wlPZtksIFYuUe6HOpWw.woff2 | fd78ed94061d91a141bf6be05559398e21a3d8aab8f7e21594d434573fcf3305 |
| zilla-slab-700.woff2 | Zilla Slab | 700 | https://fonts.gstatic.com/s/zillaslab/v12/dFa5ZfeM_74wlPZtksIFYoEf6HOpWw.woff2 | 896b6ebb8cd10aca90bdff88a5a1b648cbc0d50751740bd5a6d012007aef9ebe |
| pt-serif-caption-400.ttf | PT Serif Caption | 400 | https://raw.githubusercontent.com/google/fonts/90abd17b4f97671435798b6147b698aa9087612f/ofl/ptserifcaption/PT_Serif-Caption-Web-Regular.ttf | d17d47f44e840eee017f839b27dff3c94c7afe7743eff308d4bd11f7018b69af |

Hashes were taken with `sha256sum`.

Both families use the SIL Open Font License 1.1:

- `OFL-zilla-slab.txt`, from `https://raw.githubusercontent.com/google/fonts/6b48e8c3d3a8ca1437b404d747c0496d654cacf6/ofl/zillaslab/OFL.txt` (Copyright 2017, The Mozilla Foundation).
- `OFL-pt-serif-caption.txt`, from `https://raw.githubusercontent.com/google/fonts/90abd17b4f97671435798b6147b698aa9087612f/ofl/ptserifcaption/OFL.txt` (Copyright 2010, ParaType Ltd.).
