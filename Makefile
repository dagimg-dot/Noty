build:
	rm -rf build
	mkdir build
	cd build && meson setup .. -Dapplication_id=com.dagimg.dev.noty && meson configure -Dprefix="$(PWD)/build"

install: build
	ninja -C build && ninja -C build install

clean:
	rm -rf build

run: install
	build/bin/noty --debug

run-release:
	flatpak run com.dagimg.dev.noty

release:
	@if [ -z "$(VERSION)" ]; then \
		echo "Error: VERSION parameter is required. Use 'make release VERSION=x.y.z'"; \
		exit 1; \
	fi
	@echo "Creating and pushing tag v$(VERSION)..."
	git tag v$(VERSION)
	git push origin v$(VERSION)
	@echo "Tag v$(VERSION) created and pushed. GitHub Actions workflow should start automatically."

local-release:
	rm -rf /tmp/flatpak-build /tmp/flatpak-repo $(PWD)/dist
	mkdir -p /tmp/flatpak-build /tmp/flatpak-repo $(PWD)/dist
	cd /tmp/flatpak-build && flatpak-builder --repo=/tmp/flatpak-repo --force-clean flatpak_app $(PWD)/build-aux/flatpak/com.dagimg.dev.noty.json
	flatpak build-bundle /tmp/flatpak-repo $(PWD)/dist/noty-dev.flatpak com.dagimg.dev.noty --runtime-repo=https://flathub.org/repo/flathub.flatpakrepo
	@echo "Flatpak bundle created: dist/noty-dev.flatpak"
