build:
	rm -rf build
	mkdir build
	cd build && meson setup .. && meson configure -Dprefix="$(PWD)/build"

install: build
	ninja -C build && ninja -C build install

clean:
	rm -rf build

run: install
	build/bin/noty

release:
	@if [ -z "$(VERSION)" ]; then \
		echo "Error: VERSION parameter is required. Use 'make release VERSION=x.y.z'"; \
		exit 1; \
	fi
	@echo "Creating and pushing tag v$(VERSION)..."
	git tag v$(VERSION)
	git push origin v$(VERSION)
	@echo "Tag v$(VERSION) created and pushed. GitHub Actions workflow should start automatically."
