# Vendored fonts — sources and hashes

The console sets every piece of text in Zilla Slab (Q-B24, signed 2026-10-24) and the wordmark in PT Serif
Caption. Instrument Sans and JetBrains Mono (Google Fonts CSS2, fetched 2026-09-03) were removed on
2026-11-07 when nothing referenced them any more; they stay in git history.

## Faces (wave B, F11p0, fetched 2026-10-24)

Quinn's ruling (2026-10-24): the Knowlu wordmark is set in PT Serif Caption and every other piece of text
is Zilla Slab, in setup, onboarding and (Q-B24) the console.

**Zilla Slab** was fetched from the CSS2 endpoint
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
