// src/lib.rs
use js_sys::Float32Array;
use wasm_bindgen::prelude::*; // ← ДОБАВЛЕНО

use mapgen::{generate_heightmap, WorldGenerationParams, WorldType};

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    web_sys::console::log_1(&"MapGen WASM module initialized".into());
}

#[wasm_bindgen]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! MapGen WASM is ready.", name)
}

/// Генерирует карту высот и возвращает данные в формате, оптимальном для браузера
///
/// # Возвращает
/// JavaScript-объект с полями:
/// - `width: number` — ширина карты
/// - `height: number` — высота карты
/// - `data: Float32Array` — массив высот (0.0..1.0)
#[wasm_bindgen]
pub fn generate_heightmap_simple(seed: u32, width: u32, height: u32) -> Result<JsValue, JsValue> {
    // Генерация мира
    let params = WorldGenerationParams {
        seed: seed as u64,
        width,
        height,
        world_type: WorldType::EarthLike,
        ..Default::default()
    };

    let heightmap = generate_heightmap(
        params.seed,
        params.width,
        params.height,
        params.world_type,
        params.islands.island_density,
        &params.terrain,
    );

    // Создаём JavaScript-объект для возврата
    let result = js_sys::Object::new();

    // Устанавливаем поля (безопасно через Reflect)
    js_sys::Reflect::set(
        &result,
        &JsValue::from_str("width"),
        &JsValue::from_f64(width as f64),
    )
    .map_err(|_| JsValue::from_str("Failed to set width"))?;

    js_sys::Reflect::set(
        &result,
        &JsValue::from_str("height"),
        &JsValue::from_f64(height as f64),
    )
    .map_err(|_| JsValue::from_str("Failed to set height"))?;

    // КРИТИЧЕСКИ ВАЖНО: преобразуем Vec<f32> в Float32Array для эффективной передачи
    let data_array = Float32Array::from(&heightmap.data[..]);
    js_sys::Reflect::set(&result, &JsValue::from_str("data"), &data_array.into())
        .map_err(|_| JsValue::from_str("Failed to set data"))?;

    Ok(result.into())
}
