default:
    @just --list

build:
    mkdir -p build
    cd build && cmake ..
    cd build && cmake --build . --parallel 16

clean:
    rm -rf build

rebuild: clean build

run *args:
    ./build/simple {{args}}

try: build
    ./build/simple --version
    rm -rf test-site
    ./build/simple new test-site
    echo "Created test-site, now building..."
    ./build/simple build test-site
    echo "Build complete! Output in test-site/dist/"
    ls -la test-site/dist/

install: build
    cd build && sudo cmake --install .

debug:
    mkdir -p build-debug
    cd build-debug && cmake -DCMAKE_BUILD_TYPE=Debug ..
    cd build-debug && cmake --build .

format:
    find src include -name "*.cpp" -o -name "*.hpp" | xargs clang-format -i

check:
    find src include -name "*.cpp" -o -name "*.hpp" | xargs clang-format --dry-run --Werror
