CLANG ?= clang
WASM_LD ?= wasm-ld

hello.o: hello.c
	"$(CLANG)" -target wasm32-wasm -c $< -o $@

hello.wasm: hello.o
	"$(WASM_LD)" $< -o $@ --no-entry --export=answer
