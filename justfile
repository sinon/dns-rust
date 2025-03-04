set dotenv-load
build:
    CARGO_PROFILE_DEV_CODEGEN_BACKEND=cranelift cargo +nightly build -Zcodegen-backend
format:
        @cargo fmt --version
        cargo fmt
lint:
        @cargo clippy --version
        cargo clippy -- -D warnings
        cargo doc
test:
    cargo test

dns:
    dig @127.0.0.1 -p 2053 +noedns codecrafters.io