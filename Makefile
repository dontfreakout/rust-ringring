BINARY   := rust-ringring
VERSION  := $(shell grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')
DIST     := dist

TARGETS := \
	x86_64-unknown-linux-gnu \
	aarch64-unknown-linux-gnu \
	x86_64-apple-darwin \
	aarch64-apple-darwin

.PHONY: all clean test native $(TARGETS)

# Default: build for host platform
all: native

native:
	cargo build --release

test:
	cargo test

clean:
	cargo clean
	rm -rf $(DIST)

# Cross-compile a single target: make x86_64-unknown-linux-gnu
$(TARGETS):
	cross build --release --target $@
	@mkdir -p $(DIST)
	cp target/$@/release/$(BINARY) $(DIST)/$(BINARY)-$@

# Build all targets
dist: $(TARGETS)
	@echo "Built binaries:"
	@ls -lh $(DIST)/

# Install to ~/.claude/ for current platform
install: native
	cp target/release/$(BINARY) $(HOME)/.claude/$(BINARY)
	@echo "Installed to ~/.claude/$(BINARY)"
