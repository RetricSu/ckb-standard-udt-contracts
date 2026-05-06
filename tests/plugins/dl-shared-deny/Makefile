cur_dir = $(dir $(abspath $(firstword $(MAKEFILE_LIST))))
REPO_ROOT := $(abspath $(cur_dir)/../../..)

MODE := release
BUILD_DIR :=
CLANG := clang-17
OBJCOPY := llvm-objcopy-17
RUST_SYSROOT := $(shell rustc --print sysroot)
RUST_HOST := $(shell rustc -vV | sed -n 's/^host: //p')
LLD := $(RUST_SYSROOT)/lib/rustlib/$(RUST_HOST)/bin/gcc-ld/ld.lld
OUTPUT := dl-shared-deny

CFLAGS := --target=riscv64-unknown-linux-gnu -march=rv64imac -mabi=lp64 -fuse-ld=$(LLD) -fPIC -O3 -nostdinc -nostdlib -nostartfiles -fvisibility=hidden -fdata-sections -ffunction-sections
LDFLAGS := -Wl,-static -Wl,--gc-sections -Wl,-z,separate-code -shared

build:
	$(CLANG) $(CFLAGS) $(LDFLAGS) -o $(OUTPUT) src/udt_validate.c
	@set -eu; \
	if [ "x$(BUILD_DIR)" != "x" ]; then \
		mkdir -p "$(REPO_ROOT)/$(BUILD_DIR)"; \
		cp "$(OUTPUT)" "$(REPO_ROOT)/$(BUILD_DIR)/$(OUTPUT)"; \
		cp "$(OUTPUT)" "$(REPO_ROOT)/$(BUILD_DIR)/$(OUTPUT).debug"; \
		$(OBJCOPY) --strip-debug --strip-all "$(REPO_ROOT)/$(BUILD_DIR)/$(OUTPUT)"; \
	fi

.PHONY: build
