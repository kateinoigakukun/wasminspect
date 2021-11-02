use wasmparser::WasmFeatures;

#[derive(Default)]
pub struct Config {
    pub features: WasmFeatures,
}