.PHONY: dev run test vet lint build clean

# Live-reloading development build (development app ID, com.dagimg.dev.noty).
dev:
	go tool mygo dev

# Run once without live reload, with debug logging.
run:
	go run . --debug

test:
	go test ./...

vet:
	go vet ./...

lint: vet
	@test -z "$$(gofmt -l .)" || (gofmt -l . && echo "run gofmt -w ." && exit 1)

# Release build of the binary and Linux packages into build/.
build:
	go tool mygo build

clean:
	rm -rf ./build ./dist ./.mygo
