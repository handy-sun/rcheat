
# **rcheat**

[![build-test](https://github.com/handy-sun/rcheat/actions/workflows/build-test.yml/badge.svg)](https://github.com/handy-sun/rcheat/actions/workflows/build-test.yml)
![latest_release](https://img.shields.io/github/v/tag/handy-sun/rcheat?label=release)
[![Crates.io](https://img.shields.io/crates/v/rcheat.svg)](https://crates.io/crates/rcheat)
![Linux](https://img.shields.io/badge/-Linux-grey?logo=linux)

> *Get/modify simple variable's value in another Linux running process*

------

## 1. Dependencies

- [cargo](https://github.com/rust-lang/cargo/) >= 1.74
- [rustc](https://www.rust-lang.org/) >= 1.74

If your Linux package management(e.g. apt, yum) cannot provide the sufficient conditions for the dependencies, download a pkg form [here](https://forge.rust-lang.org/infra/archive-stable-version-installers.html)


As a developer, use [rustup](https://rust-lang.github.io/rustup/) manage rust enviroment is suitable.

## 2. Building

```shell
cargo build
```

The first build may take a long time,
because it needs to download dependent libraries from `crates.io`

**Tips:**
If download speed from `.crates.io` is too slow. use a mirror to speed up(e.g. use [rsproxy](https://rsproxy.cn)).


## 3. Usage

**NOTE: This program must be run with root privileges!**

detail arguments use `-h`
```shell
sudo /path/to/rcheat -h
```

## 4. Todo

- [ ] use log crate such as `log/envlogger`
- [ ] regex replace String.contain
- [ ] parse .debug* section
- [ ] use config.toml to reduce some inputs
- [ ] use `lua` to customized output
- [ ] search pid by process name (like linux command: `pidof/pgrep`)
- [x] if match more than 1 entry name, ask for which one to select
- [x] demangle symbols
