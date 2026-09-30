APP_ID = com.dagimg.noty
PREFIX ?= $(HOME)/.local
NOTY_VERSION = $(shell sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml)
DIST_NAME = noty-$(NOTY_VERSION)-linux-$(shell uname -m)
STAGE = target/dist/$(DIST_NAME)

.PHONY: run build test lint install uninstall dist clean bump release

run:
	NOTY_APP_ID=com.dagimg.dev.noty cargo run

build:
	cargo build --release --locked

test:
	cargo test

lint:
	cargo fmt --check
	cargo clippy --all-targets

install: build
	install -Dm755 target/release/noty -t $(DESTDIR)$(PREFIX)/bin
	install -Dm644 data/$(APP_ID).desktop -t $(DESTDIR)$(PREFIX)/share/applications
	install -Dm644 data/$(APP_ID).metainfo.xml -t $(DESTDIR)$(PREFIX)/share/metainfo
	install -Dm644 data/icons/hicolor/scalable/apps/$(APP_ID).svg -t $(DESTDIR)$(PREFIX)/share/icons/hicolor/scalable/apps
	install -Dm644 data/icons/hicolor/symbolic/apps/$(APP_ID)-symbolic.svg -t $(DESTDIR)$(PREFIX)/share/icons/hicolor/symbolic/apps

uninstall:
	rm -f $(DESTDIR)$(PREFIX)/bin/noty
	rm -f $(DESTDIR)$(PREFIX)/share/applications/$(APP_ID).desktop
	rm -f $(DESTDIR)$(PREFIX)/share/metainfo/$(APP_ID).metainfo.xml
	rm -f $(DESTDIR)$(PREFIX)/share/icons/hicolor/scalable/apps/$(APP_ID).svg
	rm -f $(DESTDIR)$(PREFIX)/share/icons/hicolor/symbolic/apps/$(APP_ID)-symbolic.svg

dist: build
	rm -rf $(STAGE)
	$(MAKE) install PREFIX=$(CURDIR)/$(STAGE)
	install -Dm644 README.md COPYING -t $(STAGE)/share/doc/noty
	mkdir -p dist
	tar -C target/dist --owner=0 --group=0 -czf dist/$(DIST_NAME).tar.gz $(DIST_NAME)

clean:
	cargo clean

bump:
	@if [ -z "$(VERSION)" ]; then \
		echo "Error: VERSION parameter is required. Use 'make release VERSION=x.y.z'"; \
		exit 1; \
	fi
	echo "Updating version to $(VERSION)..."
	sed -i '0,/^version = ".*"/s//version = "$(VERSION)"/' Cargo.toml
	cargo update --workspace --offline
	echo "Noty version updated to $(VERSION)"

release: bump
	@echo "Committing version update..."
	git add Cargo.toml Cargo.lock
	git diff --cached --quiet || git commit -m "chore: bump version to v$(VERSION)" --no-verify
	git push
	@echo "Creating and pushing tag v$(VERSION)..."
	git tag v$(VERSION)
	git push origin v$(VERSION)
	@echo "Tag v$(VERSION) created and pushed. GitHub Actions workflow should start automatically."
