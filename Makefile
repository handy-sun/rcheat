

default : all

all:
	cargo build

rel:
	cargo build --release

musl:
	cargo build --target x86_64-unknown-linux-musl --release

cln_d:
	cargo clean --target-dir target/debug

deb:
	cargo deb --target x86_64-unknown-linux-musl
