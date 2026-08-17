//! Base map generator command (engine-spec → starter VE/ignition/AFR maps).

/// Generate a base map from engine specifications
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn generate_base_map(
    cylinder_count: u8,
    displacement_cc: f64,
    injector_size_cc: f64,
    fuel_type: String,
    aspiration: String,
    stroke_type: String,
    injection_mode: String,
    ignition_mode: String,
    idle_rpm: u16,
    redline_rpm: u16,
    boost_target_kpa: Option<f64>,
    target_wot_afr: Option<f64>,
) -> Result<serde_json::Value, String> {
    use libretune_core::basemap::{
        Aspiration, EngineSpec, FuelType, IgnitionMode, InjectionMode, StrokeType,
    };

    let fuel = match fuel_type.to_lowercase().as_str() {
        "gasoline" | "petrol" => FuelType::Gasoline,
        "e85" => FuelType::E85,
        "e100" => FuelType::E100,
        "methanol" => FuelType::Methanol,
        "lpg" | "propane" => FuelType::LPG,
        _ => return Err(format!("Unknown fuel type: {}", fuel_type)),
    };

    let asp = match aspiration.to_lowercase().as_str() {
        "na" | "naturally_aspirated" => Aspiration::NA,
        "turbo" | "turbocharged" => Aspiration::Turbo,
        "supercharged" => Aspiration::Supercharged,
        _ => return Err(format!("Unknown aspiration: {}", aspiration)),
    };

    let stroke = match stroke_type.to_lowercase().as_str() {
        "four_stroke" | "4stroke" | "4" => StrokeType::FourStroke,
        "two_stroke" | "2stroke" | "2" => StrokeType::TwoStroke,
        _ => return Err(format!("Unknown stroke type: {}", stroke_type)),
    };

    let inj = match injection_mode.to_lowercase().as_str() {
        "sequential" => InjectionMode::Sequential,
        "batch" => InjectionMode::Batch,
        "simultaneous" => InjectionMode::Simultaneous,
        "throttle_body" | "tbi" => InjectionMode::ThrottleBody,
        _ => return Err(format!("Unknown injection mode: {}", injection_mode)),
    };

    let ign = match ignition_mode.to_lowercase().as_str() {
        "wasted_spark" | "wastedspark" => IgnitionMode::WastedSpark,
        "coil_on_plug" | "cop" => IgnitionMode::CoilOnPlug,
        "distributor" => IgnitionMode::Distributor,
        _ => return Err(format!("Unknown ignition mode: {}", ignition_mode)),
    };

    let spec = EngineSpec {
        cylinder_count,
        displacement_cc,
        injector_size_cc,
        fuel_type: fuel,
        aspiration: asp,
        stroke_type: stroke,
        injection_mode: inj,
        ignition_mode: ign,
        idle_rpm,
        redline_rpm,
        boost_target_kpa,
        target_wot_afr,
    };

    let base_map = libretune_core::basemap::generator::generate_base_map(&spec);

    serde_json::to_value(&base_map).map_err(|e| format!("Failed to serialize base map: {}", e))
}
