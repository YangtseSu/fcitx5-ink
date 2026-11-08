# Data

## Committed source data: Tomoe

`data/tomoe/` contains the Tomoe handwriting dictionary used by the Tegaki 0.3
Zinnia models:

| File | Language | SHA-256 |
|---|---|---|
| `handwriting-zh_CN.xml` | Simplified Chinese | `969862706c883c89ae6b945549b141e0051847930e04d8fa8a26aad2b3d4db11` |
| `handwriting-zh_TW.xml` | Traditional Chinese | `85ab37c56bcb14a54a7ae106f2504b6505cf6c9917ca8c636312153fd9d00e53` |
| `handwriting-ja.xml` | Japanese | `6f30a4f42f24dc611f1a3e4d7a220c5009bb0323f237f01f50e3be3dfefb11f8` |

These files are LGPL-2.1-or-later; `COPYING` is included. `handwriting-ja.xml`
and `handwriting-zh_CN.xml` are byte-identical to the Tomoe repository
(<https://github.com/l4u/tomoe>) at commit
`9d054a1f490368e0d45e0aff6430ce3598ab92c5`; `handwriting-zh_TW.xml` is the full
Tomoe traditional Chinese dictionary. Tegaki packaged this data for its Zinnia
models.

The XML is the training input for Zinnia and for future online neural models.
It is not a test set: Simplified and Traditional Chinese contain one sample per
character.

## Fetched models

Compiled models are not committed. `scripts/fetch-models.sh` downloads them into
`models/` and verifies the pinned sizes and SHA-256 values in
`data/manifests/`.

## Manifests

- `manifests/tomoe.json`: committed source data;
- `manifests/tegaki-models.json`: Zinnia models used by fcitx5-ink;
- `manifests/wagomu.json`: Wagomu engine and model releases for comparison;
- `manifests/makemeahanzi.json`: Make Me a Hanzi median trajectories
  (Arphic-1999), fetched by `scripts/fetch_mmh.py`.

## Licenses

- Tomoe data and Zinnia models: LGPL-2.1-or-later.
- Wagomu engine: GPL-2.0-or-later.
- Make Me a Hanzi/AnimCJK: Arphic Public License; not committed here yet.
- KanjiVG: CC BY-SA 3.0; keep it in a separate license group if added.
