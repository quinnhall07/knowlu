# New-page fonts: sources and hashes (v0.4.0 T1, note D7)

Every file here is the upstream file, byte for byte: never a per-script subset from the Google Fonts
CSS2 endpoint, never re-encoded. Fetched 2026-11-18 from `raw.githubusercontent.com` at the commits
below; each file's git blob id matched the one the GitHub contents API lists for that path. Re-check:
fetch the same path at the same commit and compare SHA-256 (`Get-FileHash -Algorithm SHA256`, lowered).

- google/fonts: `https://github.com/google/fonts`, commit `5bd87dfc525c8a0d1d8529d3684716e00c3fc59a`.
- OpenDyslexic: `https://github.com/antijingoist/opendyslexic`, branch `main`, commit
  `1824da5c0e41dc3e13ffc7f3a636dcaf695d61b7`. Shipped as upstream `.woff`: no WOFF2 conversion
  (D7 leaves that to the reviewer), so nothing here is a Modified Version.

| File | Family | Source (repository, path, commit) | Licence | RFN note | SHA-256 |
| --- | --- | --- | --- | --- | --- |
| PT_Serif-Caption-Web-Regular.ttf | PT Serif Caption | google/fonts `ofl/ptserifcaption/PT_Serif-Caption-Web-Regular.ttf` @ 5bd87dfc525c8a0d1d8529d3684716e00c3fc59a | SIL OFL 1.1 (`OFL-ptserifcaption.txt`) | Reserved Font Names "PT Sans", "PT Serif", "ParaType"; the file is unmodified, so it may carry the name | d17d47f44e840eee017f839b27dff3c94c7afe7743eff308d4bd11f7018b69af |
| AlbertSans[wght].ttf | Albert Sans | google/fonts `ofl/albertsans/AlbertSans[wght].ttf` @ 5bd87dfc525c8a0d1d8529d3684716e00c3fc59a | SIL OFL 1.1 (`OFL-albertsans.txt`) | No Reserved Font Name in its licence | 8fe5d4cf5822d7096d4d17ad781c90f97c745ac13a22be619db74966fba45fda |
| OpenDyslexic-Regular.woff | OpenDyslexic | antijingoist/opendyslexic `compiled/OpenDyslexic-Regular.woff` @ 1824da5c0e41dc3e13ffc7f3a636dcaf695d61b7 | SIL OFL 1.1 (`OFL-opendyslexic.txt`) | Reserved Font Name "OpenDyslexic"; the file is unmodified, so it may carry the name | 7465efa8054da355ee52d1c0404bab60cb2fe1eabfc1cfcecfc37679be4ecb58 |
| OpenDyslexic-Bold.woff | OpenDyslexic | antijingoist/opendyslexic `compiled/OpenDyslexic-Bold.woff` @ 1824da5c0e41dc3e13ffc7f3a636dcaf695d61b7 | SIL OFL 1.1 (`OFL-opendyslexic.txt`) | Reserved Font Name "OpenDyslexic"; the file is unmodified, so it may carry the name | 2dc180ab7df58833b64ed4a871c51adbe3c53936ca4ac3200c109b26d731075b |
| OpenDyslexic-Italic.woff | OpenDyslexic | antijingoist/opendyslexic `compiled/OpenDyslexic-Italic.woff` @ 1824da5c0e41dc3e13ffc7f3a636dcaf695d61b7 | SIL OFL 1.1 (`OFL-opendyslexic.txt`) | Reserved Font Name "OpenDyslexic"; the file is unmodified, so it may carry the name | c2c486b4b24543888bf0f4f3607019739b55f2f20e425e74a374e88335bd8962 |

## Licences

Each family's `OFL.txt`, unmodified, renamed only so the three can sit in one folder.

| File | Source (repository, path, commit) | SHA-256 |
| --- | --- | --- |
| OFL-ptserifcaption.txt | google/fonts `ofl/ptserifcaption/OFL.txt` @ 5bd87dfc525c8a0d1d8529d3684716e00c3fc59a | ddf311c28ddf5a5ad9747649837346b67bed9d356789c3072bb27dbce49e514d |
| OFL-albertsans.txt | google/fonts `ofl/albertsans/OFL.txt` @ 5bd87dfc525c8a0d1d8529d3684716e00c3fc59a | 5c856c086e8743b84932aae46ced424729a703932b601e9eb8aaeac15a617ec6 |
| OFL-opendyslexic.txt | antijingoist/opendyslexic `OFL.txt` @ 1824da5c0e41dc3e13ffc7f3a636dcaf695d61b7 | caafcccfb70fc72458fcbda812ec8f0a06cb300cbabb87eabbb30b946124394b |

Not shipped with the new page: Zilla Slab (its only app use was Ask), Instrument Sans and JetBrains
Mono (the old page's, under `static/fonts/`, which go at the swap).
