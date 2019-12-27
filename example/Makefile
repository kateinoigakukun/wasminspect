CLANG ?= clang
WASM_LD ?= wasm-ld

hello.o: hello.c
	"$(CLANG)" -target wasm32-wasm -c $< -o $@ -g

hello.wasm: hello.o
	"$(WASM_LD)" $< -o $@ --no-entry --export=answer

fizzbuzz.wasm: fizzbuzz.rs
	rustc +nightly --target wasm32-unknown-unknown -g $< -o $@
