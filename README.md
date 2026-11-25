# lib-kv-hashmap

`handl-plugin-lib-kv-hashmap` — In-memory [`HashMap`]-backed [`KeyValueStore`] implementation.

## Layout

```text
handl-packages/kv_hashmap/kv_hashmap.hnl   the .hnl surface
src/lib.rs                                  the Rust backend
scripts/hnl-gate.sh                         the shared hnl typecheck/lint/test gate
workflows/gate.yaml                                       bupha entry (handl-gates)
```

Sibling-repo deps are git dependencies (`branch = "main"`); `[patch]` tables
resolve them from sibling checkouts.

## Gate

```bash
cargo test    # the Rust suite
cargo xtask gate  # the bupha gate — delegates to scripts/hnl-gate.sh
```

## License

MIT.
