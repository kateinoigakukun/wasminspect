MAKEFILE_DIR := $(dir $(lastword $(MAKEFILE_LIST)))
WASI_SDK_DIR ?= $(MAKEFILE_DIR)/../../../.wasi-sdk
CLANG := $(WASI_SDK_DIR)/bin/clang

main.wasm: dummy.o main.o
	$(CLANG) $^ --sysroot=$(WASI_SDK_DIR)/share/wasi-sysroot -o $@
main.o: main.c
	$(CLANG) -c \
	    -g -v \
	    -target wasm32-wasi \
	    --sysroot=$(WASI_SDK_DIR)/share/wasi-sysroot \
	    -o $@ \
	    $<
dummy.o: dummy.c
	$(CLANG) -c -v \
	    -target wasm32-wasi \
	    --sysroot=$(WASI_SDK_DIR)/share/wasi-sysroot \
	    -o $@ \
	    $<
.PHONY: clean
clean:
	rm main dummy.o main.o
