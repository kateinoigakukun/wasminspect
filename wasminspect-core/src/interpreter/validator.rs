pub enum ValidationError {}

type Result<T = ()> = std::result::Result<T, ValidationError>;

struct ValidationContext {}
pub fn validate(module: &parity_wasm::elements::Module) -> Result {
    let types = module
        .type_section()
        .map(|sec| sec.types())
        .unwrap_or_default();
    Ok(())
}
