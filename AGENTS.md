# lib-kv-hashmap

`handl-plugin-lib-kv-hashmap` — in-memory `HashMap`-backed
`KeyValueStore` implementation of the `kv` contract.

## Layout

| path | role |
| --- | --- |
| `handl-packages/kv_hashmap/kv_hashmap.hnl` | the `.hnl` surface |
| `src/lib.rs` | the Rust backend |
| `atoms.yaml`, `Cargo.toml`, `xtask/` | atom registry + Rust gate |
| `pkg.hnl` | manifest — dep `kv` (`branch:main`), `modules: ["kv_hashmap"]` |
| `scripts/hnl-gate.sh`, `workflows/gate.yaml` | `.hnl` gate + CI campaign |

## Contract

- Implements the `lib-kv` `KeyValueStore` trait over `HashMap` — the
  reference in-memory backend; other backends (sled, SQLite, Redis)
  follow the same trait.
- Sibling deps are git deps on `branch:main`.

## Verification

```bash
HNL=../host-cli/target/debug/hnl scripts/hnl-gate.sh
cargo xtask gate
```

<!-- myco:agent-awareness -->

## Agent awareness

- Session start / any work discussion: run `myco status --prompt`
  before proposing next steps — know the board first.
- User describes new work → offer to capture it (`myco todo add`,
  `issue new`, `proposal new`). Don't wait to be asked.
- Picking up a task: `myco resume <ID>` is the briefing.
- Wrapping up: `myco handoff <ID>` anything resumable; keep statuses
  honest (claimed → in-progress → blocked/done).

<!-- /myco:agent-awareness -->
