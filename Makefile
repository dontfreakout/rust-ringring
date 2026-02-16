BINARY := rust-ringring
DIST   := dist
HOST   := $(shell rustc -vV | sed -n 's/host: //p')

.PHONY: all clean test install dist

all: native

native:
	cargo build --release

test:
	cargo test

clean:
	cargo clean
	rm -rf $(DIST)

# Build for host and package into dist/
dist:
	@mkdir -p $(DIST)
	cargo build --release --target $(HOST)
	cp target/$(HOST)/release/$(BINARY) $(DIST)/$(BINARY)-$(HOST)
	@echo "Built:"
	@ls -lh $(DIST)/

install: native
	cp target/release/$(BINARY) $(HOME)/.claude/$(BINARY)
	@echo "Installed to ~/.claude/$(BINARY)"

# Cross-compile a specific target, e.g.: make cross TARGET=aarch64-unknown-linux-gnu
# Requires the target toolchain: rustup target add <target>
# Requires a cross-linker (e.g. aarch64-linux-gnu-gcc) or use cargo-zigbuild
cross:
ifndef TARGET
	$(error TARGET is required, e.g. make cross TARGET=aarch64-unknown-linux-gnu)
endif
	@mkdir -p $(DIST)
	cargo build --release --target $(TARGET)
	cp target/$(TARGET)/release/$(BINARY) $(DIST)/$(BINARY)-$(TARGET)
