

default : all

all:
	cargo build

rel:
	cargo build --release

musl:
	cargo build --target x86_64-unknown-linux-musl --release

cln_d:
	cargo clean --target-dir target/debug

deb-musl:
	cargo deb --target x86_64-unknown-linux-musl

libc217-rel:
	cargo zigbuild --target x86_64-unknown-linux-gnu.2.17 --release

libc217-deb:	libc217-rel
	cargo deb --target x86_64-unknown-linux-gnu --no-build -o target/x86_64-unknown-linux-gnu/release
