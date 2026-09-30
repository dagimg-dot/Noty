APP_ID = com.dagimg.noty
PREFIX ?= $(HOME)/.local

.PHONY: run build test lint install uninstall clean flatpak-install flatpak-run bump dev-release prod-release flatpak-install-prod flatpak-run-prod release

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

clean:
	cargo clean

flatpak-install:
	flatpak install --user --or-update -y $(PWD)/dist/noty-dev.flatpak

flatpak-run:
	flatpak run com.dagimg.dev.noty

bump:
	@if [ -z "$(VERSION)" ]; then \
		echo "Error: VERSION parameter is required. Use 'make release VERSION=x.y.z'"; \
		exit 1; \
	fi
	echo "Updating version to $(VERSION)..."
	sed -i '0,/^version = ".*"/s//version = "$(VERSION)"/' Cargo.toml
	cargo update --workspace --offline
	sed -i "/noty\.git/!b;n;s/\"tag\": \"v[0-9]*\.[0-9]*\.[0-9]*\"/\"tag\": \"v$(VERSION)\"/" build-aux/flatpak/com.dagimg.noty.json
	echo "Noty version updated to $(VERSION)"

dev-release:
	rm -rf /tmp/flatpak-build /tmp/flatpak-repo $(PWD)/dist
	mkdir -p /tmp/flatpak-build /tmp/flatpak-repo $(PWD)/dist
	cd /tmp/flatpak-build && flatpak-builder --user --install-deps-from=flathub --repo=/tmp/flatpak-repo --force-clean flatpak_app $(PWD)/build-aux/flatpak/com.dagimg.dev.noty.json
	flatpak build-bundle /tmp/flatpak-repo $(PWD)/dist/noty-dev.flatpak com.dagimg.dev.noty --runtime-repo=https://flathub.org/repo/flathub.flatpakrepo
	@echo "Flatpak bundle created: dist/noty-dev.flatpak"
	@$(MAKE) flatpak-install

prod-release:
	rm -rf /tmp/flatpak-build /tmp/flatpak-repo $(PWD)/dist
	mkdir -p /tmp/flatpak-build /tmp/flatpak-repo $(PWD)/dist
	cd /tmp/flatpak-build && flatpak-builder --user --install-deps-from=flathub --repo=/tmp/flatpak-repo --force-clean flatpak_app $(PWD)/build-aux/flatpak/com.dagimg.noty.json
	flatpak build-bundle /tmp/flatpak-repo $(PWD)/dist/noty.flatpak com.dagimg.noty --runtime-repo=https://flathub.org/repo/flathub.flatpakrepo
	@echo "Flatpak bundle created: dist/noty.flatpak"
	@$(MAKE) flatpak-install-prod

flatpak-install-prod:
	flatpak install --user --or-update -y $(PWD)/dist/noty.flatpak

flatpak-run-prod:
	flatpak run com.dagimg.noty

release: bump
	@echo "Committing version update..."
	git add Cargo.toml Cargo.lock build-aux/flatpak/com.dagimg.noty.json
	git diff --cached --quiet || git commit -m "chore: bump version to v$(VERSION)" --no-verify
	git push
	@echo "Creating and pushing tag v$(VERSION)..."
	git tag v$(VERSION)
	git push origin v$(VERSION)
	@echo "Tag v$(VERSION) created and pushed. GitHub Actions workflow should start automatically."
