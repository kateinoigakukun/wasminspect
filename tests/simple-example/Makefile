MAKEFILE_DIR := $(dir $(lastword $(MAKEFILE_LIST)))
WABT_DIR ?= $(MAKEFILE_DIR)/../../.wabt
WAT2WASM := $(WABT_DIR)/wat2wasm

FIXTURES := calc.wasm

.PHONY: all
all: $(FIXTURES)

%.wasm: %.wat
	"$(WAT2WASM)" $< -o $@
.PHONY: clean
clean:
	rm *.wasm
