<font size=6>**rcheat**</font>

<font size=5> Cheat a running linux process' memory </font>

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
because it needs to download dependent libraries from `.crates.io`

**Tips:**
If download speed from `.crates.io` is too slow. use a mirror to speed up(e.g. use [rsproxy](https://rsproxy.cn)).


## 3. Usage

**NOTE: This program(intcpt) must be run with root privileges!**

detail arguments use `-h`
```shell
sudo /path/to/rcheat -h
```

