// src/lib.rs
//! MapGen Web Demo — WASM-интерфейс для браузера
//!
//! Этот модуль предоставляет функции для вызова из JavaScript.
//! Все функции должны быть помечены #[wasm_bindgen].

use js_sys::Float32Array;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use mapgen::{
    biome::assign_biomes,
    build_province_graph_with_map,
    climate::{calculate_humidity, generate_climate_maps},
    generate_heightmap,
    province::{
        generator::{generate_province_seeds, generate_provinces_from_seeds},
        water::classify_water,
    },
    region::group_provinces_into_regions,
    rivers::generate_rivers,
    ClimateSettings, IslandSettings, TerrainSettings, WorldGenerationParams, WorldType,
};

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

/// Конфигурация мира для генерации из браузера
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorldConfig {
    seed: u32,
    world_type: String,
    width: u32,
    height: u32,
    global_temperature_offset: f32,
    global_humidity_offset: f32,
    total_provinces: usize,
    elevation_power: f32,
    smooth_radius: usize,
    island_density: f32,
    min_island_size: u32,
}

/// Данные провинции для передачи в JavaScript
#[derive(Serialize)]
struct ProvinceData {
    id: u32,
    is_land: bool,
    coastal: bool,
    area: usize,
    center: [f32; 2],
}

/// Данные региона для передачи в JavaScript
#[derive(Serialize)]
struct RegionData {
    id: u32,
    name: String,
}

/// Генерирует мир с полной конфигурацией
///
/// # Параметры
/// * `config_js` — JavaScript-объект с полями:
///   - `seed: number` (u32)
///   - `worldType: string` (один из типов мира)
///   - `width: number` (u32)
///   - `height: number` (u32)
///   - `globalTemperatureOffset: number` (f32)
///   - `globalHumidityOffset: number` (f32)
///   - `totalProvinces: number` (usize)
///   - `elevationPower: number` (f32)
///   - `smoothRadius: number` (usize)
///   - `islandDensity: number` (f32)
///   - `minIslandSize: number` (u32)
///
/// # Возвращает
/// Объект с полями:
/// - `width`, `height` — размеры карты
/// - `heightmap` — Float32Array высот
/// - `biomes` — Uint32Array биомов
/// - `provinces` — Uint32Array province_id
/// - `regions` — Uint32Array region_id
/// - `provinceData` — массив данных провинций
/// - `regionData` — массив данных регионов
#[wasm_bindgen]
pub fn generate_world_with_config(config_js: JsValue) -> Result<JsValue, JsValue> {
    // Десериализуем конфигурацию из JavaScript
    let config: WorldConfig = serde_wasm_bindgen::from_value(config_js)
        .map_err(|e| JsValue::from_str(&format!("Invalid config: {}", e)))?;

    // Преобразуем тип мира из строки
    let world_type = match config.world_type.as_str() {
        "EarthLike" => WorldType::EarthLike,
        "Supercontinent" => WorldType::Supercontinent,
        "Archipelago" => WorldType::Archipelago,
        "Mediterranean" => WorldType::Mediterranean,
        "IceAgeEarth" => WorldType::IceAgeEarth,
        "DesertMediterranean" => WorldType::DesertMediterranean,
        _ => WorldType::EarthLike,
    };

    // Создаём параметры генерации с новыми настройками
    let mut params = WorldGenerationParams {
        seed: config.seed as u64,
        width: config.width,
        height: config.height,
        world_type,
        ..WorldGenerationParams::default()
    };

    // Климатические настройки
    params.climate = ClimateSettings {
        global_temperature_offset: config.global_temperature_offset,
        global_humidity_offset: config.global_humidity_offset,
        polar_amplification: 1.0,
        climate_latitude_exponent: 0.65,
    };

    // Настройки островов
    params.islands = IslandSettings {
        island_density: config.island_density,
        min_island_size: config.min_island_size,
    };

    // Настройки рельефа
    params.terrain = TerrainSettings {
        elevation_power: config.elevation_power,
        smooth_radius: config.smooth_radius,
        mountain_compression: 0.7,
        total_provinces: config.total_provinces,
    };

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
    let (temperature, winds) = generate_climate_maps(
        params.seed,
        params.width,
        params.height,
        &heightmap.data,
        params.climate.global_temperature_offset,
        params.climate.polar_amplification,
        params.climate.climate_latitude_exponent,
        sea_level,
    );

    let humidity = calculate_humidity(
        params.width,
        params.height,
        &heightmap.data,
        &winds,
        sea_level,
        params.climate.global_humidity_offset,
    );

    // 3. Назначение биомов
    let biome_map = assign_biomes(&heightmap, &temperature, &humidity, sea_level);

    // 4. Классификация воды
    let water_type = classify_water(&heightmap, sea_level);

    // 5. Генерация рек
    let _river_map = generate_rivers(&heightmap, &biome_map);

    // 6. Генерация провинций
    let land_pixels = water_type
        .iter()
        .filter(|&&t| t == mapgen::province::water::WaterType::Land)
        .count();
    let total_pixels = (params.width * params.height) as usize;
    let land_ratio = land_pixels as f32 / total_pixels as f32;

    let num_land = (config.total_provinces as f32 * 0.7).round() as usize;
    let num_sea = config.total_provinces - num_land;

    let seeds = generate_province_seeds(
        &heightmap,
        &biome_map,
        &water_type,
        num_land,
        num_sea,
        params.seed,
    );

    let (provinces, pixel_to_id) =
        generate_provinces_from_seeds(&heightmap, &biome_map, &water_type, &seeds);

    // 7. Генерация регионов
    let graph =
        build_province_graph_with_map(&provinces, &pixel_to_id, params.width, params.height);

    let regions = group_provinces_into_regions(&provinces, &graph, 8);

    // === СОЗДАНИЕ РЕЗУЛЬТАТА ===
    let result = js_sys::Object::new();

    // Высотная карта
    js_sys::Reflect::set(
        &result,
        &JsValue::from_str("heightmap"),
        &Float32Array::from(&heightmap.data[..]).into(),
    )
    .map_err(|_| JsValue::from_str("Failed to set heightmap"))?;

    // Биомы
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

    // Данные провинций
    let province_data = provinces
        .iter()
        .map(|p| ProvinceData {
            id: p.id,
            is_land: p.is_land,
            coastal: p.coastal,
            area: p.area,
            center: [p.center.0, p.center.1],
        })
        .collect::<Vec<_>>();

    js_sys::Reflect::set(
        &result,
        &JsValue::from_str("provinceData"),
        &serde_wasm_bindgen::to_value(&province_data).unwrap(),
    )
    .map_err(|_| JsValue::from_str("Failed to set provinceData"))?;

    // Данные регионов
    let region_data_js = regions
        .iter()
        .map(|r| RegionData {
            id: r.id,
            name: r.name.clone(),
        })
        .collect::<Vec<_>>();

    js_sys::Reflect::set(
        &result,
        &JsValue::from_str("regionData"),
        &serde_wasm_bindgen::to_value(&region_data_js).unwrap(),
    )
    .map_err(|_| JsValue::from_str("Failed to set regionData"))?;

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
