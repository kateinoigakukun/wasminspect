use super::value::Value;
use parity_wasm::elements::GlobalType;

pub struct GlobalInstance {
    ty: GlobalType,
    value: Value,
}

impl GlobalInstance {
    pub fn new(value: Value, ty: GlobalType) -> Self {
        Self { value, ty }
    }

    pub fn value(&self) -> Value {
        self.value
    }
}
