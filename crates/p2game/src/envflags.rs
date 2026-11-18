//! Variables de entorno de diagnóstico, leídas una vez por nombre.
//!
//! `std::env::var` toma un bloqueo global del proceso y reserva memoria en cada llamada. Varios
//! caminos calientes (la IA por tick, el bucle de disparo, cada frame de la UI) las consultaban
//! miles de veces por segundo. Aquí se consultan una vez y el resultado queda en caché.
//! Las variables no cambian durante la ejecución (launch.rs las fija al arrancar).

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

static CACHE: LazyLock<Mutex<HashMap<&'static str, Option<String>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn cached(name: &'static str) -> Option<String> {
    let mut map = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    map.entry(name).or_insert_with(|| std::env::var(name).ok()).clone()
}

/// Equivale a `std::env::var_os(name).is_some()`, con caché.
pub fn set(name: &'static str) -> bool {
    cached(name).is_some()
}

/// Equivale a `std::env::var(name).ok()`, con caché.
pub fn get(name: &'static str) -> Option<String> {
    cached(name)
}
