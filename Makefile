MAKEFILE_DIR := $(dir $(lastword $(MAKEFILE_LIST)))
WABT_DIR ?= $(MAKEFILE_DIR)/.wabt
WASI_SDK_DIR ?= $(MAKEFILE_DIR)/.wasi-sdk

ifeq  ($(shell uname),Darwin)
WABT_DOWNLOAD_URL="https://github.com/WebAssembly/wabt/releases/download/1.0.12/wabt-1.0.12-osx.tar.gz"
WASI_SDK_DOWNLOAD_URL="https://github.com/CraneStation/wasi-sdk/releases/download/wasi-sdk-8/wasi-sdk-8.0-macos.tar.gz"
else
WABT_DOWNLOAD_URL="https://github.com/WebAssembly/wabt/releases/download/1.0.12/wabt-1.0.12-linux.tar.gz"
WASI_SDK_DOWNLOAD_URL="https://github.com/CraneStation/wasi-sdk/releases/download/wasi-sdk-8/wasi-sdk-8.0-linux.tar.gz"
endif

.PHONY: fixtures
fixtures: .wabt .wasi-sdk
	cd tests/simple-example; make all

.wabt:
	mkdir -p $(WABT_DIR) && cd $(WABT_DIR) && \
            curl -L $(WABT_DOWNLOAD_URL) | tar xz --strip-components 1
.wasi-sdk:
	mkdir -p $(WASI_SDK_DIR) && cd $(WASI_SDK_DIR) && \
            curl -L $(WASI_SDK_DOWNLOAD_URL) | tar xz --strip-components 1
