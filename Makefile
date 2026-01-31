configure:
	mkdir -p build
	cd build && meson setup .. -Dapplication_id=com.dagimg.dev.noty && meson configure -Dprefix="$(PWD)/build"

install: configure
	ninja -C build && ninja -C build install

clean:
	rm -rf build

run: install
	build/bin/noty --debug

crun: clean run

lint:
	ruff check . --fix
	ruff format .

flatpak-install:
	flatpak install --user --or-update -y $(PWD)/dist/noty-dev.flatpak

flatpak-run:
	flatpak run com.dagimg.dev.noty --debug

bump:
	@if [ -z "$(VERSION)" ]; then \
		echo "Error: VERSION parameter is required. Use 'make release VERSION=x.y.z'"; \
		exit 1; \
	fi
	echo "Updating version to $(VERSION)..."
	sed -i "s/version: '[0-9]*\.[0-9]*\.[0-9]*'/version: '$(VERSION)'/" meson.build
	sed -i "/noty\.git/!b;n;s/\"tag\": \"v[0-9]*\.[0-9]*\.[0-9]*\"/\"tag\": \"v$(VERSION)\"/" build-aux/flatpak/com.dagimg.noty.json
	echo "Noty version updated to $(VERSION)"

dev-release:
	rm -rf /tmp/flatpak-build /tmp/flatpak-repo $(PWD)/dist
	mkdir -p /tmp/flatpak-build /tmp/flatpak-repo $(PWD)/dist
	cd /tmp/flatpak-build && flatpak-builder --repo=/tmp/flatpak-repo --force-clean flatpak_app $(PWD)/build-aux/flatpak/com.dagimg.dev.noty.json
	flatpak build-bundle /tmp/flatpak-repo $(PWD)/dist/noty-dev.flatpak com.dagimg.dev.noty --runtime-repo=https://flathub.org/repo/flathub.flatpakrepo
	@echo "Flatpak bundle created: dist/noty-dev.flatpak"
	@$(MAKE) flatpak-install

prod-release:
	rm -rf /tmp/flatpak-build /tmp/flatpak-repo $(PWD)/dist
	mkdir -p /tmp/flatpak-build /tmp/flatpak-repo $(PWD)/dist
	cd /tmp/flatpak-build && flatpak-builder --repo=/tmp/flatpak-repo --force-clean flatpak_app $(PWD)/build-aux/flatpak/com.dagimg.noty.json
	flatpak build-bundle /tmp/flatpak-repo $(PWD)/dist/noty.flatpak com.dagimg.noty --runtime-repo=https://flathub.org/repo/flathub.flatpakrepo
	@echo "Flatpak bundle created: dist/noty.flatpak"
	@$(MAKE) flatpak-install-prod

flatpak-install-prod:
	flatpak install --user --or-update -y $(PWD)/dist/noty.flatpak

flatpak-run-prod:
	flatpak run com.dagimg.noty

release: bump
	@echo "Committing version update..."
	git add meson.build build-aux/flatpak/com.dagimg.noty.json
	git commit -m "chore: bump version to v$(VERSION)" --no-verify
	git push
	@echo "Creating and pushing tag v$(VERSION)..."
	git tag v$(VERSION)
	git push origin v$(VERSION)
	@echo "Tag v$(VERSION) created and pushed. GitHub Actions workflow should start automatically."
