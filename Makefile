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
