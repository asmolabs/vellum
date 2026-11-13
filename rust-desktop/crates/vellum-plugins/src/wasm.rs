//! Exécuteur sandboxé de plugins WebAssembly (WASM) basé sur `wasmi`.
//!
//! Permet d'exécuter des modules `.wasm` tiers sans accès système (pas de WASI,
//! pas de réseau, pas de système de fichiers) avec un budget strict de carburant
//! (`fuel`) et une limite de mémoire linéaire.

use serde::{Deserialize, Serialize};
use std::path::Path;
use vellum_domain::VellumError;
use wasmi::{Config, Engine, Linker, Module, Store, StoreLimits, StoreLimitsBuilder};

/// Limites d'exécution imposées à un plugin WASM sandboxé.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmExecutionLimits {
    /// Budget maximal d'instructions/carburant (`fuel`) par appel.
    pub max_fuel: u64,
    /// Taille maximale de mémoire linéaire allouable (en octets).
    pub max_memory_bytes: usize,
    /// Nombre maximal d'instances de tables.
    pub max_tables: usize,
}

impl Default for WasmExecutionLimits {
    fn default() -> Self {
        Self {
            max_fuel: 1_000_000,
            max_memory_bytes: 16 * 1024 * 1024, // 16 MiB
            max_tables: 4,
        }
    }
}

/// Manifeste JSON optionnel décrivant un plugin WASM (`*.wasm.json`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmPluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    pub wasm_file: String,
    #[serde(default = "default_entrypoint")]
    pub entrypoint: String,
    #[serde(default)]
    pub limits: WasmExecutionLimits,
}

fn default_entrypoint() -> String {
    "run".to_string()
}

/// Résultat d'une exécution WASM sandboxée avec métriques de consommation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WasmExecutionResult<T> {
    pub value: T,
    pub fuel_consumed: u64,
}

struct SandboxState {
    limits: StoreLimits,
}

/// Exécuteur sandboxé pour plugins WebAssembly (`wasmi`).
#[derive(Debug, Clone)]
pub struct WasmPluginRunner {
    limits: WasmExecutionLimits,
}

impl Default for WasmPluginRunner {
    fn default() -> Self {
        Self::new(WasmExecutionLimits::default())
    }
}

impl WasmPluginRunner {
    /// Crée un nouveau runner WASM avec les limites de sécurité spécifiées.
    pub fn new(limits: WasmExecutionLimits) -> Self {
        Self { limits }
    }

    fn build_engine(&self) -> Engine {
        let mut config = Config::default();
        config.consume_fuel(true);
        Engine::new(&config)
    }

    fn build_store(&self, engine: &Engine) -> Result<Store<SandboxState>, VellumError> {
        let limits = StoreLimitsBuilder::new()
            .memory_size(self.limits.max_memory_bytes)
            .tables(self.limits.max_tables)
            .instances(1)
            .build();
        let mut store = Store::new(engine, SandboxState { limits });
        store.limiter(|state| &mut state.limits);
        store
            .set_fuel(self.limits.max_fuel)
            .map_err(|e| VellumError::Plugin(format!("WASM fuel setup error: {e}")))?;
        Ok(store)
    }

    /// Valide un module WASM (binaire `.wasm` ou texte `.wat` si activé) sans l'exécuter.
    pub fn validate_module(&self, wasm_bytes: &[u8]) -> Result<(), VellumError> {
        let engine = self.build_engine();
        Module::new(&engine, wasm_bytes)
            .map(|_| ())
            .map_err(|e| VellumError::Plugin(format!("Invalid WASM module: {e}")))
    }

    /// Charge et valide un manifeste JSON de plugin WASM depuis le disque.
    pub fn load_manifest(path: &Path) -> Result<WasmPluginManifest, VellumError> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            VellumError::Plugin(format!("Cannot read WASM manifest {}: {e}", path.display()))
        })?;
        serde_json::from_str(&content).map_err(|e| {
            VellumError::Plugin(format!("Invalid WASM manifest {}: {e}", path.display()))
        })
    }

    /// Exécute une fonction exportée `(i32) -> i32` dans un sandbox isolé.
    pub fn run_i32_transform(
        &self,
        wasm_bytes: &[u8],
        fn_name: &str,
        input: i32,
    ) -> Result<WasmExecutionResult<i32>, VellumError> {
        let engine = self.build_engine();
        let module = Module::new(&engine, wasm_bytes)
            .map_err(|e| VellumError::Plugin(format!("WASM compile error: {e}")))?;
        let mut store = self.build_store(&engine)?;
        let linker = Linker::<SandboxState>::new(&engine);
        let instance = linker
            .instantiate_and_start(&mut store, &module)
            .map_err(|e| VellumError::Plugin(format!("WASM instantiate error: {e}")))?;
        let func = instance
            .get_typed_func::<i32, i32>(&store, fn_name)
            .map_err(|e| VellumError::Plugin(format!("WASM export '{fn_name}' not found: {e}")))?;

        let value = func
            .call(&mut store, input)
            .map_err(|e| VellumError::Plugin(format!("WASM execution trapped: {e}")))?;

        let remaining = store.get_fuel().unwrap_or(0);
        let fuel_consumed = self.limits.max_fuel.saturating_sub(remaining);
        Ok(WasmExecutionResult {
            value,
            fuel_consumed,
        })
    }

    /// Exécute un hook textuel/JSON via la mémoire linéaire WASM.
    ///
    /// Convention ABI du module :
    /// - `memory` : mémoire linéaire exportée
    /// - `alloc(len: i32) -> i32` : alloue `len` octets et retourne le pointeur
    /// - `<hook_fn>(ptr: i32, len: i32) -> i64` : retourne `(out_ptr << 32) | out_len`
    pub fn run_json_hook(
        &self,
        wasm_bytes: &[u8],
        alloc_fn: &str,
        hook_fn: &str,
        input_json: &str,
    ) -> Result<WasmExecutionResult<String>, VellumError> {
        let engine = self.build_engine();
        let module = Module::new(&engine, wasm_bytes)
            .map_err(|e| VellumError::Plugin(format!("WASM compile error: {e}")))?;
        let mut store = self.build_store(&engine)?;
        let linker = Linker::<SandboxState>::new(&engine);
        let instance = linker
            .instantiate_and_start(&mut store, &module)
            .map_err(|e| VellumError::Plugin(format!("WASM instantiate error: {e}")))?;

        let memory = instance
            .get_memory(&store, "memory")
            .ok_or_else(|| VellumError::Plugin("WASM export 'memory' not found".to_string()))?;
        let alloc = instance
            .get_typed_func::<i32, i32>(&store, alloc_fn)
            .map_err(|e| {
                VellumError::Plugin(format!("WASM export '{alloc_fn}' not found: {e}"))
            })?;
        let hook = instance
            .get_typed_func::<(i32, i32), i64>(&store, hook_fn)
            .map_err(|e| {
                VellumError::Plugin(format!("WASM export '{hook_fn}' not found: {e}"))
            })?;

        let input_bytes = input_json.as_bytes();
        let input_len = i32::try_from(input_bytes.len())
            .map_err(|_| VellumError::Plugin("WASM input too large".to_string()))?;
        let in_ptr = alloc
            .call(&mut store, input_len)
            .map_err(|e| VellumError::Plugin(format!("WASM alloc trapped: {e}")))?;
        if in_ptr < 0 {
            return Err(VellumError::Plugin("WASM alloc returned negative pointer".to_string()));
        }
        memory
            .write(&mut store, in_ptr as usize, input_bytes)
            .map_err(|e| VellumError::Plugin(format!("WASM memory write error: {e}")))?;

        let packed = hook
            .call(&mut store, (in_ptr, input_len))
            .map_err(|e| VellumError::Plugin(format!("WASM hook '{hook_fn}' trapped: {e}")))?;

        let out_ptr = ((packed as u64) >> 32) as usize;
        let out_len = ((packed as u64) & 0xFFFF_FFFF) as usize;
        let mut out_buf = vec![0u8; out_len];
        memory
            .read(&store, out_ptr, &mut out_buf)
            .map_err(|e| VellumError::Plugin(format!("WASM memory read error: {e}")))?;

        let value = String::from_utf8(out_buf)
            .map_err(|e| VellumError::Plugin(format!("WASM output is not valid UTF-8: {e}")))?;
        let remaining = store.get_fuel().unwrap_or(0);
        let fuel_consumed = self.limits.max_fuel.saturating_sub(remaining);

        Ok(WasmExecutionResult {
            value,
            fuel_consumed,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_minimal_wasm_and_reject_invalid() {
        let runner = WasmPluginRunner::default();
        let minimal_wasm = [0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
        assert!(runner.validate_module(&minimal_wasm).is_ok());
        assert!(runner.validate_module(b"not-a-wasm-module").is_err());
    }

    #[test]
    fn test_run_i32_transform_and_fuel_metering() {
        let runner = WasmPluginRunner::default();
        let wat = r#"
            (module
                (func (export "double_plus_one") (param i32) (result i32)
                    local.get 0
                    i32.const 2
                    i32.mul
                    i32.const 1
                    i32.add
                )
            )
        "#;
        let res = runner
            .run_i32_transform(wat.as_bytes(), "double_plus_one", 20)
            .expect("WASM transform should succeed");
        assert_eq!(res.value, 41);
        assert!(res.fuel_consumed > 0);
    }

    #[test]
    fn test_fuel_exhaustion_stops_infinite_loop() {
        let runner = WasmPluginRunner::new(WasmExecutionLimits {
            max_fuel: 500,
            ..Default::default()
        });
        let wat = r#"
            (module
                (func (export "spin") (param i32) (result i32)
                    (loop $l
                        br $l
                    )
                    i32.const 0
                )
            )
        "#;
        let err = runner
            .run_i32_transform(wat.as_bytes(), "spin", 0)
            .expect_err("infinite loop must trap on fuel exhaustion");
        assert!(err.to_string().contains("trapped"));
    }

    #[test]
    fn test_run_json_hook_echo_via_linear_memory() {
        let runner = WasmPluginRunner::default();
        let wat = r#"
            (module
                (memory (export "memory") 1)
                (func (export "alloc") (param $len i32) (result i32)
                    i32.const 1024
                )
                (func (export "echo") (param $ptr i32) (param $len i32) (result i64)
                    local.get $ptr
                    i64.extend_i32_u
                    i64.const 32
                    i64.shl
                    local.get $len
                    i64.extend_i32_u
                    i64.or
                )
            )
        "#;
        let payload = r#"{"title":"Dune","author":"Frank Herbert"}"#;
        let res = runner
            .run_json_hook(wat.as_bytes(), "alloc", "echo", payload)
            .expect("JSON hook should succeed");
        assert_eq!(res.value, payload);
        assert!(res.fuel_consumed > 0);
    }
}

