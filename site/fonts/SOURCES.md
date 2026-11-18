# knowlu.com fonts: sources and hashes (v0.4.0 T4, note §9 M11)

Re-check any row: fetch its source and compare SHA-256 (`Get-FileHash -Algorithm SHA256`, lowered).

- **PT Serif Caption** is the whole upstream file from google/fonts (`https://github.com/google/fonts`,
  commit `5bd87dfc525c8a0d1d8529d3684716e00c3fc59a`), fetched 2026-11-18; its git blob id matched the
  one the GitHub contents API lists for that path. It replaces the site's earlier
  `PTSerifCaption-400.woff2`, the Google Fonts CSS2 `latin` subset (SHA-256
  `c869134ca7e6116a0f4e1b0972897f2fde14a1b2780c18e41e6fcf52bdbe0bd3`): a subset is a Modified
  Version under the OFL, and this licence reserves the name "PT Serif".
- **Albert Sans** and **Zilla Slab** are the files the live site already served, copied unchanged. On
  2026-11-18 each was byte-identical to the Google Fonts CSS2 `latin` file at the URL in its row.
  Those are per-script subsets, which the OFL allows here because neither licence reserves a font
  name, so a Modified Version may keep it.

| File | Family | Source | Licence | RFN note | SHA-256 |
| --- | --- | --- | --- | --- | --- |
| PT_Serif-Caption-Web-Regular.ttf | PT Serif Caption | google/fonts `ofl/ptserifcaption/PT_Serif-Caption-Web-Regular.ttf` @ 5bd87dfc525c8a0d1d8529d3684716e00c3fc59a | SIL OFL 1.1 (`OFL-ptserifcaption.txt`) | Reserved Font Names "PT Sans", "PT Serif", "ParaType"; the file is unmodified, so it may carry the name | d17d47f44e840eee017f839b27dff3c94c7afe7743eff308d4bd11f7018b69af |
| AlbertSans-latin.woff2 | Albert Sans | Google Fonts CSS2 `latin`, `https://fonts.gstatic.com/s/albertsans/v4/i7dOIFdwYjGaAMFtZd_QA1ZbYFc.woff2` (weights 400 to 700) | SIL OFL 1.1 (`OFL-albertsans.txt`) | No Reserved Font Name in its licence | c2fb6fda415c7bf5926c15aa98f759da45cdc5ac4ac6925a72a84db4eb3862a3 |
| ZillaSlab-400.woff2 | Zilla Slab | Google Fonts CSS2 `latin`, `https://fonts.gstatic.com/s/zillaslab/v12/dFa6ZfeM_74wlPZtksIFajo6_Q.woff2` (weight 400) | SIL OFL 1.1 (`OFL-zillaslab.txt`) | No Reserved Font Name in its licence | ab2607313bc04e82a53aca235853a0b83463e99b3ae88b17cc963e371f8fe91c |
| ZillaSlab-500.woff2 | Zilla Slab | Google Fonts CSS2 `latin`, `https://fonts.gstatic.com/s/zillaslab/v12/dFa5ZfeM_74wlPZtksIFYskZ6HOpWw.woff2` (weight 500) | SIL OFL 1.1 (`OFL-zillaslab.txt`) | No Reserved Font Name in its licence | 993eac5cf2c8439fc1d781e6de24bbac2885b089e9f170795b8bcad280ca4790 |
| ZillaSlab-600.woff2 | Zilla Slab | Google Fonts CSS2 `latin`, `https://fonts.gstatic.com/s/zillaslab/v12/dFa5ZfeM_74wlPZtksIFYuUe6HOpWw.woff2` (weight 600) | SIL OFL 1.1 (`OFL-zillaslab.txt`) | No Reserved Font Name in its licence | fd78ed94061d91a141bf6be05559398e21a3d8aab8f7e21594d434573fcf3305 |

## Licences

Each family's `OFL.txt` from google/fonts, unmodified, renamed only so the three can sit in one folder.

| File | Source (repository, path, commit) | SHA-256 |
| --- | --- | --- |
| OFL-ptserifcaption.txt | google/fonts `ofl/ptserifcaption/OFL.txt` @ 5bd87dfc525c8a0d1d8529d3684716e00c3fc59a | ddf311c28ddf5a5ad9747649837346b67bed9d356789c3072bb27dbce49e514d |
| OFL-albertsans.txt | google/fonts `ofl/albertsans/OFL.txt` @ 5bd87dfc525c8a0d1d8529d3684716e00c3fc59a | 5c856c086e8743b84932aae46ced424729a703932b601e9eb8aaeac15a617ec6 |
| OFL-zillaslab.txt | google/fonts `ofl/zillaslab/OFL.txt` @ 5bd87dfc525c8a0d1d8529d3684716e00c3fc59a | 240b385d3ec8c8d5c1d4af2bbba6ac72bac0309a80c1a8ff112c168643cc6811 |
