MAKEFILE_DIR := $(dir $(lastword $(MAKEFILE_LIST)))
WABT_DIR ?= $(MAKEFILE_DIR)/.wabt

ifeq  ($(shell uname),Darwin)
WABT_DOWNLOAD_URL="https://github.com/WebAssembly/wabt/releases/download/1.0.12/wabt-1.0.12-osx.tar.gz"
else
WABT_DOWNLOAD_URL="https://github.com/WebAssembly/wabt/releases/download/1.0.12/wabt-1.0.12-linux.tar.gz"
endif

.PHONY: fixtures
fixtures: .wabt
	cd tests/simple-example; make all

.wabt:
	mkdir -p $(WABT_DIR) && cd $(WABT_DIR) && \
            curl -L $(WABT_DOWNLOAD_URL) | tar xz --strip-components 1
