//! # handl-lib-kv-hashmap
//!
//! In-memory [`HashMap`]-backed [`KeyValueStore`] implementation.
//!
//! Provides [`HashKeyValueStore`] — a thread-safe, in-memory key-value store
//! suitable for testing, development, and ephemeral workloads.
//!
//! This crate has been migrated to the `op`/`handl` declaration form
//! (ADR-0077 Phase 4) and to the kernel op ABI (`handl-ops`): the
//! kv-hashmap interface is declared in
//! `handl-packages/kv_hashmap/kv_hashmap.hnl` via `op`/`handl`
//! declarations, and the Rust kv logic is registered as ops in a kernel
//! [`handl_ops::OpRegistry`] via [`kv_hashmap_op_registry`]. The host
//! imports the registry via `OpRegistry::import_kernel`; module/plugin
//! wrappers live in consumer repos (e.g. `handl-kit-stdlib`).
//!
//! ## Usage
//!
//! ```ignore
//! use handl_plugin_module::ModuleRuntime;
//! use handl_plugin_lib_kv::KeyValueStore;
//! use handl_plugin_lib_kv_hashmap::HashKeyValueStore;
//!
//! // Host-side: build a module whose `kv.*` ops dispatch to
//! // `kv_hashmap_op_registry()` (see `handl-kit-stdlib`).
//! ```

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use handl_ops::{Op, OpContext, OpError, OpRegistry};
use handl_plugin_lib_kv::{KeyValueStore, KvError};

/// The handl source declaring the kv_hashmap `op` interface — the single
/// source of truth for the kv-hashmap API surface. Exposed for consumer
/// repos that build the host module/plugin wrappers.
pub const KV_HASHMAP_HNL: &str = include_str!("../handl-packages/kv_hashmap/kv_hashmap.hnl");

/// The op names declared in `kv_hashmap.hnl`.
pub const KV_HASHMAP_OPS: &[&str] = &[
    "kv.get",
    "kv.set",
    "kv.remove",
    "kv.clear",
    "kv.has",
    "kv.size",
];

/// The `atoms.yaml` atom spec (P-013 T-151) — vendored for consumers that
/// build the spec-driven registry (`atoms::module_atoms` +
/// [`kv_impls`]) instead of [`kv_hashmap_op_registry`].
pub const KV_HASHMAP_ATOMS: &str = include_str!("../atoms.yaml");

/// In-memory key-value store backed by [`HashMap`].
///
/// Thread-safe via `Mutex`. For persistent storage, consider a future
/// sled or SQLite backend.
#[derive(Default)]
pub struct HashKeyValueStore {
    data: Mutex<HashMap<String, String>>,
}

impl HashKeyValueStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with(key: impl Into<String>, value: impl Into<String>) -> Self {
        let mut data = HashMap::new();
        data.insert(key.into(), value.into());
        HashKeyValueStore {
            data: Mutex::new(data),
        }
    }
}

impl KeyValueStore for HashKeyValueStore {
    fn get(&self, key: &str) -> Result<Option<String>, KvError> {
        Ok(self.data.lock().unwrap().get(key).cloned())
    }

    fn set(&self, key: &str, value: &str) -> Result<(), KvError> {
        self.data.lock().unwrap().insert(key.into(), value.into());
        Ok(())
    }

    fn remove(&self, key: &str) -> Result<(), KvError> {
        self.data.lock().unwrap().remove(key);
        Ok(())
    }

    fn clear(&self) -> Result<(), KvError> {
        self.data.lock().unwrap().clear();
        Ok(())
    }

    fn has(&self, key: &str) -> Result<bool, KvError> {
        Ok(self.data.lock().unwrap().contains_key(key))
    }

    fn size(&self) -> Result<usize, KvError> {
        Ok(self.data.lock().unwrap().len())
    }
}

// ---------------------------------------------------------------------------
// Op impls (registered in the kernel OpRegistry)
// ---------------------------------------------------------------------------

/// Shared store backing the op impls. Each op holds a clone of this `Arc`
/// so all ops operate on the same underlying store.
fn shared_store() -> Arc<HashKeyValueStore> {
    Arc::new(HashKeyValueStore::default())
}

/// Require exactly one string argument from the call args.
fn require_one_string(args: &[serde_json::Value]) -> Result<&str, OpError> {
    let arg = args.first().ok_or(OpError::TypeMismatch(format!(
        "expected {} args, got {}",
        1, 0,
    )))?;
    arg.as_str().ok_or(OpError::TypeMismatch(format!(
        "expected {}, got {}",
        "string", "non-string",
    )))
}

/// Require exactly two string arguments from the call args.
fn require_two_strings(args: &[serde_json::Value]) -> Result<(&str, &str), OpError> {
    if args.len() < 2 {
        return Err(OpError::TypeMismatch(format!(
            "expected {} args, got {}",
            2,
            args.len(),
        )));
    }
    let key = args[0].as_str().ok_or(OpError::TypeMismatch(format!(
        "expected {}, got {}",
        "string", "non-string",
    )))?;
    let value = args[1].as_str().ok_or(OpError::TypeMismatch(format!(
        "expected {}, got {}",
        "string", "non-string",
    )))?;
    Ok((key, value))
}

/// Op impl for `kv.get` — retrieve a value by key.
pub struct KvGet {
    store: Arc<HashKeyValueStore>,
}

impl Op for KvGet {
    fn call(
        &self,
        args: &[serde_json::Value],
        _ctx: &mut dyn OpContext,
    ) -> Result<serde_json::Value, OpError> {
        let key = require_one_string(args)?;
        let value = self.store.get(key)?;
        Ok(match value {
            Some(v) => serde_json::Value::from(v),
            None => serde_json::Value::Null,
        })
    }
}

/// Op impl for `kv.set` — set a key-value pair.
pub struct KvSet {
    store: Arc<HashKeyValueStore>,
}

impl Op for KvSet {
    fn call(
        &self,
        args: &[serde_json::Value],
        _ctx: &mut dyn OpContext,
    ) -> Result<serde_json::Value, OpError> {
        let (key, value) = require_two_strings(args)?;
        self.store.set(key, value)?;
        Ok(serde_json::Value::Null)
    }
}

/// Op impl for `kv.remove` — remove a key.
pub struct KvRemove {
    store: Arc<HashKeyValueStore>,
}

impl Op for KvRemove {
    fn call(
        &self,
        args: &[serde_json::Value],
        _ctx: &mut dyn OpContext,
    ) -> Result<serde_json::Value, OpError> {
        let key = require_one_string(args)?;
        self.store.remove(key)?;
        Ok(serde_json::Value::Null)
    }
}

/// Op impl for `kv.clear` — clear all entries.
pub struct KvClear {
    store: Arc<HashKeyValueStore>,
}

impl Op for KvClear {
    fn call(
        &self,
        _args: &[serde_json::Value],
        _ctx: &mut dyn OpContext,
    ) -> Result<serde_json::Value, OpError> {
        self.store.clear()?;
        Ok(serde_json::Value::Null)
    }
}

/// Op impl for `kv.has` — check if a key exists.
pub struct KvHas {
    store: Arc<HashKeyValueStore>,
}

impl Op for KvHas {
    fn call(
        &self,
        args: &[serde_json::Value],
        _ctx: &mut dyn OpContext,
    ) -> Result<serde_json::Value, OpError> {
        let key = require_one_string(args)?;
        self.store
            .has(key)
            .map(serde_json::Value::from)
            .map_err(Into::into)
    }
}

/// Op impl for `kv.size` — return the number of entries.
pub struct KvSize {
    store: Arc<HashKeyValueStore>,
}

impl Op for KvSize {
    fn call(
        &self,
        _args: &[serde_json::Value],
        _ctx: &mut dyn OpContext,
    ) -> Result<serde_json::Value, OpError> {
        self.store
            .size()
            .map(|n| serde_json::Value::from(serde_json::json!(n)))
            .map_err(Into::into)
    }
}

// ---------------------------------------------------------------------------
// Op Registry
// ---------------------------------------------------------------------------

/// Build the kernel [`handl_ops::OpRegistry`] for kv-hashmap, registering
/// the op impls under their op names.
///
/// This registry maps:
/// - `"kv.get"` → [`KvGet`]
/// - `"kv.set"` → [`KvSet`]
/// - `"kv.remove"` → [`KvRemove`]
/// - `"kv.clear"` → [`KvClear`]
/// - `"kv.has"` → [`KvHas`]
/// - `"kv.size"` → [`KvSize`]
///
/// All impls share a single [`HashKeyValueStore`] instance so that state
/// persists across calls within the same registry. The host imports this
/// registry via `handl_plugin_lib_handl::OpRegistry::import_kernel`.
pub fn kv_hashmap_op_registry() -> OpRegistry {
    let store = shared_store();
    let mut registry = OpRegistry::new();
    registry.register(
        "kv.get",
        Arc::new(KvGet {
            store: store.clone(),
        }),
    );
    registry.register(
        "kv.set",
        Arc::new(KvSet {
            store: store.clone(),
        }),
    );
    registry.register(
        "kv.remove",
        Arc::new(KvRemove {
            store: store.clone(),
        }),
    );
    registry.register(
        "kv.clear",
        Arc::new(KvClear {
            store: store.clone(),
        }),
    );
    registry.register(
        "kv.has",
        Arc::new(KvHas {
            store: store.clone(),
        }),
    );
    registry.register("kv.size", Arc::new(KvSize { store }));
    registry
}

/// `impl:` table for `atoms.yaml` (`module: kv`) — P-013 T-151 spec-driven
/// registry path. Same shared-store contract as [`kv_hashmap_op_registry`]:
/// one [`HashKeyValueStore`] `Arc` cloned into every impl so state
/// persists across calls. Keys match the spec's `impl:` fields.
pub fn kv_impls() -> Vec<(&'static str, Arc<dyn Op>)> {
    let store = shared_store();
    vec![
        ("kv_get", Arc::new(KvGet { store: store.clone() })),
        ("kv_set", Arc::new(KvSet { store: store.clone() })),
        (
            "kv_remove",
            Arc::new(KvRemove { store: store.clone() }),
        ),
        ("kv_clear", Arc::new(KvClear { store: store.clone() })),
        ("kv_has", Arc::new(KvHas { store: store.clone() })),
        ("kv_size", Arc::new(KvSize { store })),
    ]
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use handl_ops::{OpContext, OpError};

    use super::HashKeyValueStore;
    use handl_plugin_lib_kv::kv_op_registry;

    /// Ops under test never touch the context; a stub that fails loudly is
    /// enough.
    struct StubCtx;

    impl OpContext for StubCtx {
        fn eval_expr(
            &mut self,
            _expr: &str,
            _vars: &[(String, serde_json::Value)],
        ) -> Result<serde_json::Value, OpError> {
            Err(OpError::Other(
                "eval_expr not supported in tests".to_string(),
            ))
        }

        fn try_call_value(
            &mut self,
            _callee: &serde_json::Value,
            _args: &[serde_json::Value],
        ) -> Option<Result<serde_json::Value, OpError>> {
            None
        }
    }

    fn call(
        registry: &handl_ops::OpRegistry,
        name: &str,
        args: &[serde_json::Value],
    ) -> serde_json::Value {
        registry
            .get(name)
            .unwrap_or_else(|| panic!("{name} registered"))
            .call(args, &mut StubCtx)
            .expect("call succeeds")
    }

    fn s(v: &str) -> serde_json::Value {
        serde_json::Value::from(v.to_string())
    }

    #[test]
    fn set_and_get() {
        let registry = kv_op_registry(HashKeyValueStore::default());
        call(&registry, "kv.set", &[s("key"), s("value")]);
        assert_eq!(
            call(&registry, "kv.get", &[s("key")]).as_str(),
            Some("value")
        );
    }

    #[test]
    fn get_missing_returns_null() {
        let registry = kv_op_registry(HashKeyValueStore::default());
        assert!(call(&registry, "kv.get", &[s("nope")]).is_null());
    }

    #[test]
    fn has() {
        let registry = kv_op_registry(HashKeyValueStore::with("k", "v"));
        assert_eq!(call(&registry, "kv.has", &[s("k")]).as_bool(), Some(true));
        assert_eq!(call(&registry, "kv.has", &[s("x")]).as_bool(), Some(false));
    }

    #[test]
    fn remove() {
        let registry = kv_op_registry(HashKeyValueStore::with("k", "v"));
        call(&registry, "kv.remove", &[s("k")]);
        assert_eq!(call(&registry, "kv.has", &[s("k")]).as_bool(), Some(false));
    }

    #[test]
    fn clear() {
        let registry = kv_op_registry(HashKeyValueStore::with("a", "1"));
        call(&registry, "kv.clear", &[]);
        assert_eq!(call(&registry, "kv.size", &[]).as_f64(), Some(0.0));
    }

    #[test]
    fn size() {
        let registry = kv_op_registry(HashKeyValueStore::with("a", "1"));
        assert_eq!(call(&registry, "kv.size", &[]).as_f64(), Some(1.0));
    }
}
