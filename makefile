TOML_FILE := Cargo.toml

VERSION := $(shell sed -n 's/^version *= *"\(.*\)"/\1/p' $(TOML_FILE))

PATCH_VERSION := $(shell \
	echo $(VERSION) | awk -F. '{printf "%d.%d.%d", $$1, $$2, $$3+1}' \
)

NEW_VERSION ?= $(PATCH_VERSION)

.PHONY: version tag release

version:
	@echo "Current version: $(VERSION)"
	# Update version in Cargo.toml
	@sed -i.bak 's/^version *= *".*"/version = "$(NEW_VERSION)"/' $(TOML_FILE)
	@rm -f $(TOML_FILE).bak
	@echo "Release version: $(NEW_VERSION)"
	cargo check
tag:
	@git tag -a v$(NEW_VERSION) -m "Release v$(NEW_VERSION)"
	@git push origin v$(NEW_VERSION)

package:
	@echo packaging crate
	git add $(TOML_FILE) Cargo.lock
	@git commit -m "Bump version to v$(NEW_VERSION)"
	@git push
	echo added git
	@cargo package

release: version package tag
	@echo "Creating GitHub release v$(NEW_VERSION)"
	@gh release create v$(NEW_VERSION) \
		--title "v$(NEW_VERSION)" \
		--notes "Release v$(NEW_VERSION)"
	## necessary to publish to crates.io, Currently disabled to avoid accidental publish
	# @echo "Creating crate release v$(NEW_VERSION)"
	# @cargo publish

lint: 
	cargo clippy --benches --examples --tests -- -D warnings 
fmt:
	cargo clippy --workspace --all-targets --tests --fix --allow-dirty -- -D warnings
	cargo fmt --all
build:
	cargo build
test:
	cargo test -- --nocapture
all: fmt lint build test
