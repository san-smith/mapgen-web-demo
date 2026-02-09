// src/lib.rs
//! MapGen Web Demo — WASM-интерфейс для браузера
//!
//! Этот модуль предоставляет функции для вызова из JavaScript.
//! Все функции должны быть помечены #[wasm_bindgen].

use js_sys::Float32Array;
use serde::Deserialize;
use wasm_bindgen::prelude::*;

use mapgen::{generate_heightmap, WorldGenerationParams, WorldType};

/// Инициализация WASM-модуля
///
/// Вызывается автоматически при загрузке модуля в браузере.
/// Устанавливает хук для вывода паник в консоль браузера.
#[wasm_bindgen(start)]
pub fn main() {
    // Выводим паники в консоль браузера вместо молчаливого падения
    console_error_panic_hook::set_once();

    // Лог для отладки
    web_sys::console::log_1(&"MapGen WASM module initialized".into());
}

/// Тестовая функция для проверки работы WASM
///
/// # Пример вызова из JavaScript:
/// ```javascript
/// const result = await wasm.greet("World");
/// console.log(result); // "Hello, World!"
/// ```
#[wasm_bindgen]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! MapGen WASM is ready.", name)
}

/// Генерирует карту высот с минимальными параметрами
///
/// # Параметры
/// * `seed` — сид для детерминированной генерации (u32)
/// * `width` — ширина карты в пикселях
/// * `height` — высота карты в пикселях
///
/// # Возвращает
/// JavaScript-объект с полями:
/// - `width: number` — ширина карты
/// - `height: number` — высота карты
/// - ` Float32Array` — массив высот (0.0..1.0)
#[wasm_bindgen]
pub fn generate_heightmap_simple(seed: u32, width: u32, height: u32) -> Result<JsValue, JsValue> {
    // Создаём параметры генерации с настройками по умолчанию
    let params = WorldGenerationParams {
        seed: seed as u64,
        width,
        height,
        world_type: WorldType::EarthLike,
        ..Default::default()
    };

    // Генерируем карту высот
    let heightmap = generate_heightmap(
        params.seed,
        params.width,
        params.height,
        params.world_type,
        params.islands.island_density,
        &params.terrain,
    );

    // Создаём результат для возврата в JavaScript
    let result = js_sys::Object::new();

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

    // Преобразуем данные в Float32Array для эффективной передачи
    let data_array = Float32Array::from(&heightmap.data[..]);
    js_sys::Reflect::set(&result, &JsValue::from_str("data"), &data_array.into())
        .map_err(|_| JsValue::from_str("Failed to set data"))?;

    Ok(result.into())
}

/// Конфигурация мира для генерации из браузера
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorldConfig {
    seed: u32,
    width: u32,
    height: u32,
    global_temperature_offset: f32,
    global_humidity_offset: f32,
}

/// Генерирует мир с полной конфигурацией
///
/// # Параметры
/// * `config_js` — JavaScript-объект с полями:
///   - `seed: number` (u32)
///   - `width: number` (u32)
///   - `height: number` (u32)
///   - `globalTemperatureOffset: number` (f32)
///   - `globalHumidityOffset: number` (f32)
///
/// # Возвращает
/// Объект с полями `width`, `height`, `data` (Float32Array)
#[wasm_bindgen]
pub fn generate_world_with_config(config_js: JsValue) -> Result<JsValue, JsValue> {
    // Десериализуем конфигурацию из JavaScript
    let config: WorldConfig = serde_wasm_bindgen::from_value(config_js)
        .map_err(|e| JsValue::from_str(&format!("Invalid config: {}", e)))?;

    // Создаём параметры генерации
    let mut params = WorldGenerationParams::default();
    params.seed = config.seed as u64;
    params.width = config.width;
    params.height = config.height;

    // Настраиваем климат
    params.climate.global_temperature_offset = config.global_temperature_offset;
    params.climate.global_humidity_offset = config.global_humidity_offset;

    // Генерируем карту высот
    let heightmap = generate_heightmap(
        params.seed,
        params.width,
        params.height,
        params.world_type,
        params.islands.island_density,
        &params.terrain,
    );

    // Создаём результат для возврата в JavaScript
    let result = js_sys::Object::new();

    js_sys::Reflect::set(
        &result,
        &JsValue::from_str("width"),
        &JsValue::from_f64(config.width as f64),
    )
    .map_err(|_| JsValue::from_str("Failed to set width"))?;

    js_sys::Reflect::set(
        &result,
        &JsValue::from_str("height"),
        &JsValue::from_f64(config.height as f64),
    )
    .map_err(|_| JsValue::from_str("Failed to set height"))?;

    let data_array = Float32Array::from(&heightmap.data[..]);
    js_sys::Reflect::set(&result, &JsValue::from_str("data"), &data_array.into())
        .map_err(|_| JsValue::from_str("Failed to set data"))?;

    Ok(result.into())
}
