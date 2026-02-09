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
    let config: WorldConfig = serde_wasm_bindgen::from_value(config_js)
        .map_err(|e| JsValue::from_str(&format!("Invalid config: {}", e)))?;

    // Создаём параметры генерации
    let mut params = WorldGenerationParams::default();
    params.seed = config.seed as u64;
    params.width = config.width;
    params.height = config.height;
    params.climate.global_temperature_offset = config.global_temperature_offset;
    params.climate.global_humidity_offset = config.global_humidity_offset;

    // === ПОЛНЫЙ КОНВЕЙЕР ГЕНЕРАЦИИ ===
    let sea_level = 0.5;

    // 1. Генерация карты высот
    let heightmap = generate_heightmap(
        params.seed,
        params.width,
        params.height,
        params.world_type,
        params.islands.island_density,
        &params.terrain,
    );

    // 2. Генерация климата
    let (temperature, winds) = mapgen::climate::generate_climate_maps(
        params.seed,
        params.width,
        params.height,
        &heightmap.data,
        params.climate.global_temperature_offset,
        params.climate.polar_amplification,
        params.climate.climate_latitude_exponent,
        sea_level,
    );

    let humidity = mapgen::climate::calculate_humidity(
        params.width,
        params.height,
        &heightmap.data,
        &winds,
        sea_level,
        params.climate.global_humidity_offset,
    );

    // 3. Назначение биомов
    let biome_map = mapgen::biome::assign_biomes(&heightmap, &temperature, &humidity, sea_level);

    // 4. Классификация воды
    let water_type = mapgen::province::water::classify_water(&heightmap, sea_level);

    // 5. Генерация рек
    let river_map = mapgen::rivers::generate_rivers(&heightmap, &biome_map);

    // 6. Генерация провинций
    let land_pixels = water_type
        .iter()
        .filter(|&&t| t == mapgen::province::water::WaterType::Land)
        .count();
    let total_pixels = (params.width * params.height) as usize;
    let land_ratio = land_pixels as f32 / total_pixels as f32;

    let num_land = (params.terrain.total_provinces as f32 * 0.7).round() as usize;
    let num_sea = params.terrain.total_provinces - num_land;

    let seeds = mapgen::province::generator::generate_province_seeds(
        &heightmap,
        &biome_map,
        &water_type,
        num_land,
        num_sea,
        params.seed,
    );

    let (provinces, pixel_to_id) = mapgen::province::generator::generate_provinces_from_seeds(
        &heightmap,
        &biome_map,
        &water_type,
        &seeds,
    );

    // 7. Генерация регионов
    let graph = mapgen::province::graph::build_province_graph_with_map(
        &provinces,
        &pixel_to_id,
        params.width,
        params.height,
    );

    let regions = mapgen::region::group_provinces_into_regions(&provinces, &graph, 8);

    // === СОЗДАНИЕ РЕЗУЛЬТАТА ===
    let result = js_sys::Object::new();

    // Высотная карта
    js_sys::Reflect::set(
        &result,
        &JsValue::from_str("heightmap"),
        &Float32Array::from(&heightmap.data[..]).into(),
    )
    .map_err(|_| JsValue::from_str("Failed to set heightmap"))?;

    // Биомы (целые числа)
    let biome_data: Vec<u32> = biome_map.data.iter().map(|&b| b as u32).collect();
    js_sys::Reflect::set(
        &result,
        &JsValue::from_str("biomes"),
        &js_sys::Uint32Array::from(&biome_data[..]).into(),
    )
    .map_err(|_| JsValue::from_str("Failed to set biomes"))?;

    // Провинции
    js_sys::Reflect::set(
        &result,
        &JsValue::from_str("provinces"),
        &js_sys::Uint32Array::from(&pixel_to_id[..]).into(),
    )
    .map_err(|_| JsValue::from_str("Failed to set provinces"))?;

    // Регионы
    let mut region_data = vec![0u32; pixel_to_id.len()];
    for y in 0..params.height as usize {
        for x in 0..params.width as usize {
            let idx = y * params.width as usize + x;
            let province_id = pixel_to_id[idx];
            if let Some(region) = regions
                .iter()
                .find(|r| r.province_ids.contains(&province_id))
            {
                region_data[idx] = region.id;
            }
        }
    }
    js_sys::Reflect::set(
        &result,
        &JsValue::from_str("regions"),
        &js_sys::Uint32Array::from(&region_data[..]).into(),
    )
    .map_err(|_| JsValue::from_str("Failed to set regions"))?;

    // Метаданные
    js_sys::Reflect::set(
        &result,
        &JsValue::from_str("width"),
        &JsValue::from_f64(params.width as f64),
    )
    .map_err(|_| JsValue::from_str("Failed to set width"))?;

    js_sys::Reflect::set(
        &result,
        &JsValue::from_str("height"),
        &JsValue::from_f64(params.height as f64),
    )
    .map_err(|_| JsValue::from_str("Failed to set height"))?;

    Ok(result.into())
}
