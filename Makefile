# We cannot use $(shell pwd), which will return unix path format on Windows,
# making it hard to use.
cur_dir = $(dir $(abspath $(lastword $(MAKEFILE_LIST))))

TOP := $(cur_dir)
MODE ?= release
# RUSTFLAGS that are likely to be tweaked by developers. For example,
# while we enable debug logs by default here, some might want to strip them
# for minimal code size / consumed cycles.
ifeq (debug,$(MODE))
	DEFAULT_CUSTOM_RUSTFLAGS := -C debug-assertions
else
	DEFAULT_CUSTOM_RUSTFLAGS :=
endif
CUSTOM_RUSTFLAGS ?= $(DEFAULT_CUSTOM_RUSTFLAGS)
# Additional cargo args to append here. For example, one can use
# make test CARGO_ARGS="-- --nocapture" so as to inspect data emitted to
# stdout in unit tests
CARGO_ARGS ?=
# Tweak this to change the clang version to use for building C code. By default
# we use a bash script with somes heuristics to find clang in current system.
CLANG := $(shell $(TOP)/scripts/find_clang)
# When this is set, a single contract will be built instead of all contracts
CONTRACT :=
# Simulator build mode:
# - auto: build simulator when package exists; otherwise skip with message
# - true: require simulator package and fail if missing
# - false: skip simulator build
SIM_BUILD ?= auto
# By default, we would clean build/{release,debug} folder first, in case old
# contracts are mixed together with new ones, if for some reason you want to
# revert this behavior, you can change this to anything other than true
CLEAN_BUILD_DIR_FIRST := true
BUILD_DIR := build/$(MODE)
CONTRACTS := sudt sudt-meta access-list xudt xudt-meta
TEST_PLUGINS := dl-allow dl-deny spawn-allow spawn-deny authority-spawn-allow authority-spawn-deny
TEST_SHARED_PLUGINS := dl-shared-allow dl-shared-deny authority-dl-allow authority-dl-deny
TEST_REQUIRED_BINARIES := $(addprefix $(BUILD_DIR)/,$(CONTRACTS) $(TEST_PLUGINS) $(TEST_SHARED_PLUGINS))

ifeq (release,$(MODE))
	MODE_ARGS := --release
endif

# Pass setups to child make processes
export CUSTOM_RUSTFLAGS
export TOP
export CARGO_ARGS
export MODE
export CLANG
export BUILD_DIR

default: build test

build:
	@if [ "x$(CLEAN_BUILD_DIR_FIRST)" = "xtrue" ]; then \
		echo "Cleaning $(BUILD_DIR) directory..."; \
		rm -rf $(BUILD_DIR); \
	fi
	mkdir -p $(BUILD_DIR)
	@set -eu; \
	if [ "x$(CONTRACT)" = "x" ]; then \
		for contract in $(CONTRACTS); do \
			if [ "$$contract" = "sudt-meta" ]; then \
				sudt_hash=$$(python3 $(TOP)/scripts/ckb-data-hash $(BUILD_DIR)/sudt); \
				SUDT_CODE_HASH=$$sudt_hash $(MAKE) -e -C contracts/$$contract build; \
			elif [ "$$contract" = "xudt" ]; then \
				access_list_hash=$$(python3 $(TOP)/scripts/ckb-data-hash $(BUILD_DIR)/access-list); \
				ACCESS_LIST_CODE_HASH=$$access_list_hash $(MAKE) -e -C contracts/$$contract build; \
			elif [ "$$contract" = "xudt-meta" ]; then \
				xudt_hash=$$(python3 $(TOP)/scripts/ckb-data-hash $(BUILD_DIR)/xudt); \
				access_list_hash=$$(python3 $(TOP)/scripts/ckb-data-hash $(BUILD_DIR)/access-list); \
				XUDT_CODE_HASH=$$xudt_hash ACCESS_LIST_CODE_HASH=$$access_list_hash $(MAKE) -e -C contracts/$$contract build; \
			else \
				$(MAKE) -e -C contracts/$$contract build; \
			fi; \
		done; \
		for plugin in $(TEST_PLUGINS); do \
			$(MAKE) -e -C tests/plugins/$$plugin build; \
		done; \
		for plugin in $(TEST_SHARED_PLUGINS); do \
			$(MAKE) -e -C tests/plugins/$$plugin build; \
		done; \
		for crate in $(wildcard crates/*); do \
			cargo build -p $$(basename $$crate) $(MODE_ARGS) $(CARGO_ARGS); \
		done; \
		case "$(SIM_BUILD)" in \
			true|auto) \
				for sim in $(wildcard native-simulators/*); do \
					cargo build -p $$(basename $$sim) $(CARGO_ARGS); \
				done; \
				;; \
			false) \
				echo "Skipping simulator builds (SIM_BUILD=false)."; \
				;; \
			*) \
				echo "Invalid SIM_BUILD='$(SIM_BUILD)'. Expected auto, true, or false."; \
				exit 1; \
				;; \
		esac; \
	else \
		if [ "$(CONTRACT)" = "sudt-meta" ]; then \
			$(MAKE) -e -C contracts/sudt build; \
			sudt_hash=$$(python3 $(TOP)/scripts/ckb-data-hash $(BUILD_DIR)/sudt); \
			SUDT_CODE_HASH=$$sudt_hash $(MAKE) -e -C contracts/sudt-meta build; \
		elif [ "$(CONTRACT)" = "xudt-meta" ]; then \
			$(MAKE) -e -C contracts/access-list build; \
			access_list_hash=$$(python3 $(TOP)/scripts/ckb-data-hash $(BUILD_DIR)/access-list); \
			ACCESS_LIST_CODE_HASH=$$access_list_hash $(MAKE) -e -C contracts/xudt build; \
			xudt_hash=$$(python3 $(TOP)/scripts/ckb-data-hash $(BUILD_DIR)/xudt); \
			XUDT_CODE_HASH=$$xudt_hash ACCESS_LIST_CODE_HASH=$$access_list_hash $(MAKE) -e -C contracts/xudt-meta build; \
		elif [ "$(CONTRACT)" = "sudt" ]; then \
			$(MAKE) -e -C contracts/sudt build; \
			sudt_hash=$$(python3 $(TOP)/scripts/ckb-data-hash $(BUILD_DIR)/sudt); \
			SUDT_CODE_HASH=$$sudt_hash $(MAKE) -e -C contracts/sudt-meta build; \
		elif [ "$(CONTRACT)" = "xudt" ]; then \
			$(MAKE) -e -C contracts/access-list build; \
			access_list_hash=$$(python3 $(TOP)/scripts/ckb-data-hash $(BUILD_DIR)/access-list); \
			ACCESS_LIST_CODE_HASH=$$access_list_hash $(MAKE) -e -C contracts/xudt build; \
		else \
			$(MAKE) -e -C contracts/$(CONTRACT) build; \
		fi; \
		sim_package="$(CONTRACT)-sim"; \
		case "$(SIM_BUILD)" in \
			true) \
				cargo build -p $$sim_package $(CARGO_ARGS); \
				;; \
			false) \
				echo "Skipping simulator build for $$sim_package (SIM_BUILD=false)."; \
				;; \
			auto) \
				if cargo metadata --no-deps --format-version 1 | grep -q "\"name\":\"$$sim_package\""; then \
					cargo build -p $$sim_package $(CARGO_ARGS); \
				else \
					echo "Skipping simulator build for $$sim_package (package not found in workspace)."; \
				fi; \
				;; \
			*) \
				echo "Invalid SIM_BUILD='$(SIM_BUILD)'. Expected auto, true, or false."; \
				exit 1; \
				;; \
		esac; \
	fi;

# Run a single make task for a specific contract. For example:
#
# make run CONTRACT=stack-reorder TASK=adjust_stack_size STACK_SIZE=0x200000
TASK :=
run:
	$(MAKE) -e -C contracts/$(CONTRACT) $(TASK)

# test, check, clippy and fmt here are provided for completeness,
# there is nothing wrong invoking cargo directly instead of make.
test:
	@set -eu; \
	missing=0; \
	for artifact in $(TEST_REQUIRED_BINARIES); do \
		if [ ! -f "$$artifact" ]; then \
			if [ $$missing -eq 0 ]; then \
				echo "Missing build artifacts required by tests:"; \
			fi; \
			echo "  - $$artifact"; \
			missing=1; \
		fi; \
	done; \
	if [ $$missing -eq 1 ]; then \
		echo "Hint: run 'make build MODE=$(MODE)' to generate all contract binaries, then rerun 'make test'."; \
		exit 2; \
	fi
	cargo test $(CARGO_ARGS)

check:
	cargo check $(CARGO_ARGS)

clippy:
	cargo clippy $(CARGO_ARGS)

fmt:
	cargo fmt $(CARGO_ARGS)

# Arbitrary cargo command is supported here. For example:
#
# make cargo CARGO_CMD=expand CARGO_ARGS="--ugly"
#
# Invokes:
# cargo expand --ugly
CARGO_CMD :=
cargo:
	cargo $(CARGO_CMD) $(CARGO_ARGS)

clean:
	rm -rf build
	cargo clean

TEMPLATE_TYPE := --git
TEMPLATE_REPO := https://github.com/cryptape/ckb-script-templates
CRATE :=
TEMPLATE := contract
DESTINATION := contracts
generate:
	@set -eu; \
	if [ "x$(CRATE)" = "x" ]; then \
		cargo generate $(TEMPLATE_TYPE) $(TEMPLATE_REPO) $(TEMPLATE) \
			--destination $(DESTINATION); \
		GENERATED_DIR=$$(ls -dt $(DESTINATION)/* | head -n 1); \
		if [ -f "$$GENERATED_DIR/.cargo-generate/tests.rs" ]; then \
			cat $$GENERATED_DIR/.cargo-generate/tests.rs >> tests/src/tests.rs; \
			rm -rf $$GENERATED_DIR/.cargo-generate/; \
		fi; \
		sed "s,@@INSERTION_POINT@@,@@INSERTION_POINT@@\n  \"$$GENERATED_DIR\"\,," Cargo.toml > Cargo.toml.new; \
		mv Cargo.toml.new Cargo.toml; \
	else \
		cargo generate $(TEMPLATE_TYPE) $(TEMPLATE_REPO) $(TEMPLATE) \
			--destination $(DESTINATION) \
			--name $(CRATE); \
		if [ -f "$(DESTINATION)/$(CRATE)/.cargo-generate/tests.rs" ]; then \
			cat $(DESTINATION)/$(CRATE)/.cargo-generate/tests.rs >> tests/src/tests.rs; \
			rm -rf $(DESTINATION)/$(CRATE)/.cargo-generate/; \
		fi; \
		sed '\|@@INSERTION_POINT@@|s|$$|\n  "$(DESTINATION)/$(CRATE)",|' Cargo.toml > Cargo.toml.new; \
		mv Cargo.toml.new Cargo.toml; \
	fi;

generate-native-simulator:
	@set -eu; \
	cargo generate $(TEMPLATE_TYPE) $(TEMPLATE_REPO) native-simulator \
		-n $(CRATE)-sim \
		--destination native-simulators; \
	sed '/@@INSERTION_POINT@@/s/$$/\n  "native-simulators\/$(CRATE)-sim",/' Cargo.toml > Cargo.toml.new; \
	mv Cargo.toml.new Cargo.toml;

prepare:
	rustup target add riscv64imac-unknown-none-elf

# Generate checksum info for reproducible build
CHECKSUM_FILE := build/checksums-$(MODE).txt
checksum: build
	shasum -a 256 build/$(MODE)/* > $(CHECKSUM_FILE)

.PHONY: build test check clippy fmt cargo clean prepare checksum
