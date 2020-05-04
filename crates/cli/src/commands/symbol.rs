#[cfg(feature = "swift-extension")]
use wasminspect_swift_runtime::demangle;

pub fn demangle_symbol(symbol: &str) -> &str {
    if is_swift_symbol(symbol) {
        demangle_swift_symbol(symbol)
    } else {
        symbol
    }
}

fn is_swift_symbol(symbol: &str) -> bool {
    symbol.starts_with("$s")
}

#[cfg(feature = "swift-extension")]
fn demangle_swift_symbol(symbol: &str) -> &str {
    demangle(symbol).unwrap_or(symbol)
}
#[cfg(not(feature = "swift-extension"))]
fn demangle_swift_symbol(symbol: &str) -> &str { symbol }