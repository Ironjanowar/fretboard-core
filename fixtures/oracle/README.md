# Frozen oracle fixtures

Exports of the pinned Elixir web application, produced by the exporter in
`tools/oracle/` and consumed by every core test. Ordinary builds and CI read
these files and never need Elixir or the web repository.

## Provenance

- Source repository: `Ironjanowar/fretboard`, commit `2daa8c665efa268942dda352691f39d78db42512`.
- Exporter revision hash (`exporter_sha256`): `231a0f41ffbf0547cbeffa71b344cf1318fe65df833161e565b8957b6fbe5528`.
- Elixir `1.19.5`, Erlang/OTP `27`, fixture schema version `1`, case seed `0`.
- Run host: `Linux 6.18.40-1-lts x86_64`; run recorded in `run-info.json`, which is never committed.
- Files: 28 fixture files plus `manifest.json`; total records: 22099.

## Regeneration

```sh
./scripts/export-oracle.sh --source /workspace/repos/fretboard \
  --commit 2daa8c665efa268942dda352691f39d78db42512 --out dist/oracle-run-a
python3 tools/oracle/check_export.py --left dist/oracle-run-a --right dist/oracle-run-b \
  --expect-source-commit 2daa8c665efa268942dda352691f39d78db42512
```

Two runs must be byte-identical and the checker must exit 0 before the fixtures are
copied here. Regeneration is a reviewed task, never a way to make a failing test green.

## Files

| File | Records | Bytes | SHA-256 |
|---|---:|---:|---|
| `analyzer.jsonl` | 34 | 106314 | `43a6cd27269522ae7c2b251195745c8710483c4fe26cf8c8165ca2594771ff07` |
| `catalogs.json` | 16 | 81727 | `bdab299bfba4643a7d4fd947b8f9c25d1fe1fbaf167c06dc55c44595e15bd429` |
| `chords.jsonl` | 2543 | 705224 | `0d1b1aefcbd8518507edf2844cd05e21eba0c06a6ceae86bbe1ee9bf5ecd2573` |
| `identify-01.jsonl` | 933 | 16734009 | `17e647ec6af42e4ebe502d7d962717b34feb7ec75d4241e86cf8f53c94ed1dee` |
| `identify-02.jsonl` | 629 | 16725868 | `42613a01f79dfb51a74d033c7d1db6bbd0561532c7f68cd568a30ba18277e919` |
| `identify-03.jsonl` | 377 | 16724918 | `30f6a76cfbb80b05c8a224409e3c0aefacc376703d3e7da7f9e8ff3aa7e7a761` |
| `identify-04.jsonl` | 594 | 16772474 | `012db1121f55e88d623b127c22d76c8e80cfd30860e0a6de86233c7fedd8d2e0` |
| `identify-05.jsonl` | 402 | 16741635 | `04e7d66ddd4e1e25d7760a22829bc93348dc2f65da184fe543c856586f6e1160` |
| `identify-06.jsonl` | 423 | 16731612 | `9c1b6605865761650c12b8a3ae4d0ddffde77f1c055d2d7e273ccae8f410f680` |
| `identify-07.jsonl` | 323 | 16747498 | `8d3a370f5946aa58690cb8e40c43bcd895e0b32a9afd6a0c1fcbcacee89d3291` |
| `identify-08.jsonl` | 257 | 16734012 | `5dd9c8d4194b2a18cc3be2996a3c2c31379d026d9b0b399ecfc805135ffbc967` |
| `identify-09.jsonl` | 574 | 16771795 | `c5e5f9c3ddad7b8fc3b2d28dc437c038c64a9fabe5485854b7e7a10c588527b9` |
| `identify-10.jsonl` | 887 | 16772867 | `6f424aa029758c45d27fb61d88b47e01812a8f8768650e2115ad82b2b68ffbde` |
| `identify-11.jsonl` | 889 | 16769095 | `c80f4d0215c5d394d8e400fd30437473269c656cc22c9ed1f5d8087d5a0c9e8e` |
| `identify-12.jsonl` | 2501 | 16776657 | `c84cce83748cbf0b81034ea71f43dbc90ad8a279093a8b96624a1619da6d84e7` |
| `identify-13.jsonl` | 3667 | 16775765 | `6219b0c0b6aab766ce5fd2cb431b058ac39ceac592f79c7ff27fe09aacc01da5` |
| `identify-14.jsonl` | 2583 | 16775410 | `12d012ff96a83d92c078679b09af921e2f82379167fffb2d845c4578d3169eb9` |
| `identify-15.jsonl` | 654 | 9811650 | `739e29ae6e232b509838308bc974390268f4319ab9383478828ee73756792bd4` |
| `key-groups.jsonl` | 16 | 16052 | `90135f6063dc4522b53a0ada3bd3f8ef3203916a04b3153f7b656576d999c06e` |
| `keys.jsonl` | 17 | 103322 | `e3db795ae54206154b8cf39fc196da428db29f969b1cf7d3911bcdbf32db51b2` |
| `multi-keys.jsonl` | 15 | 12157 | `e78b05b5cf049f97d01b4f9e2564fea2c5eb6adf5b28a8d734c211ae195ae7a1` |
| `page-events.jsonl` | 49 | 90864 | `dc68c36d1c9609e5d559056d20de229359877a2b6a896fc6f74b05eb2482160b` |
| `page-params.jsonl` | 104 | 67448 | `df92664459b4b4e895d0762189ac7ea9c9d2d9d5cf4c470ad6193cffa47adaca` |
| `progressions.jsonl` | 1180 | 435640 | `6e188cfd0bb20cb5af642b2e195e6dd2837bd72f518b31cc80b0f1eea4d47c10` |
| `query-transport.jsonl` | 36 | 17471 | `f9564aede656d0dac739d92c606227a4ad546885538908ae451580ed7f66257c` |
| `scales.jsonl` | 555 | 202311 | `e7c33fac210c9cc3718de278d071723f9cb583837f271916a41ba3a63447a51c` |
| `surfaces.jsonl` | 525 | 167093 | `a98649b16fd520f32fdfa63356b8d98c45cec308d62fc87be3faca9fe822f281` |
| `tunings.jsonl` | 1316 | 459375 | `41253c41aaa50aff24c21b1a019e5d33bf438751c3f854fcff01391e911088e7` |

The identify fixtures are split into contiguous 16 MiB shards (`identify-01.jsonl` …);
the concatenation in shard order is the frozen identify stream, and no record was
dropped, reordered or rewritten by the split. Total on-disk weight is roughly 236 MB,
almost all of it identify.

## Read this before editing anything under `tools/oracle/`

The interpretation order inside tied results is not reproducible from the pinned source
alone: it follows the Erlang VM's atom-table state at run time, and the exporter's own
source participates in that state. Measurements, the reproduction commands and the
consequences for `Contract.D03` are recorded in `docs/decisions.md`. Practically: any
edit to an exporter source file can silently permute `identify-*.jsonl` and
`analyzer.jsonl`, so such an edit must regenerate, re-review and re-freeze those files
and update the recorded `exporter_sha256`.
