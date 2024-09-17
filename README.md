
# **rcheat**

[![build-test](https://github.com/handy-sun/rcheat/actions/workflows/build-test.yml/badge.svg)](https://github.com/handy-sun/rcheat/actions/workflows/build-test.yml)
![latest_release](https://img.shields.io/github/v/tag/handy-sun/rcheat?label=release)
[![Crates.io](https://img.shields.io/crates/v/rcheat.svg)](https://crates.io/crates/rcheat)
![Linux](https://img.shields.io/badge/-Linux-grey?logo=linux)

> *Get/modify simple variable's value in another Linux running process*

------

<!-- vscode-markdown-toc -->
* 1. [Installation](#Installation)
	* 1.1. [Via cargo](#Viacargo)
	* 1.2. [Build src](#Buildsrc)
		* 1.2.1. [Dependencies](#Dependencies)
		* 1.2.2. [Building](#Building)
* 2. [Simple Example](#SimpleExample)
* 3. [Todo](#Todo)

<!-- vscode-markdown-toc-config
	numbering=true
	autoSave=true
	/vscode-markdown-toc-config -->
<!-- /vscode-markdown-toc -->

##  1. <a name='Installation'></a>Installation

###  1.1. <a name='Viacargo'></a>Via cargo

<details>
<summary>The way to install cargo</summary>

- can be obtained using [rustup](https://rust-lang.github.io/rustup/)(Recommond)
- use Linux package management(e.g. apt, yum, dnf, pacman)
- download a offline tarball from [forge.rust-lang.org](https://forge.rust-lang.org/infra/archive-stable-version-installers.html)
</details>

In order to install, just run the following command

```sh
cargo install --force rcheat
```

This will install cargo-make in your `~/.cargo/bin`.
Make sure to add `~/.cargo/bin` directory to your `PATH` variable.
You will have a executable available: *`rcheat`*

###  1.2. <a name='Buildsrc'></a>Build src

<!-- <a name="dependencies"></a> -->
####  1.2.1. <a name='Dependencies'></a>Dependencies

- [cargo](https://github.com/rust-lang/cargo/) >= 1.74
- [rustc](https://www.rust-lang.org/) >= 1.74

Suggest using the latest version

####  1.2.2. <a name='Building'></a>Building

```shell
git clone https://github.com/handy-sun/rcheat.git
cd rcheat
cargo build
```

You will have a executable available: *`./target/debug/rcheat`*

**Tips:**
If download speed from `crates.io` is too slow. use a mirror to speed up(e.g. use [rsproxy](https://rsproxy.cn)).


<a name="simple-example"></a>
##  2. <a name='SimpleExample'></a>Simple Example

for example, a `C` source file `onlyc.c` with some global variables:

```c
#include <unistd.h>

const char sc_sig_arr[][6] = { " ", "HUP", "INT", "QUIT", "ILL", "TRAP", "IOT", "BUS", "FPE", "KILL" };
const char techs[] = "\x02str.wa : ? !\ndaw\r21";
struct DemoStru {
    int int32;
    short uint16;
};
struct DemoStru structure;

int main() {
    structure.int32 = 0x7ffe8092;
    structure.uint16 = 0x321b;
    while (1) {
        sleep(30);
    }
    return 0;
}
```

Then compile and run it:
```sh
gcc onlyc.c -o onlyc && ./onlyc
```

Get pid of `onlyc`(e.g. use command: `pidof`) and use `rcheat` with `-p` option:
**NOTE: This program must be run with root privileges!**

```sh
pidof onlyc
# output: 13725
sudo rcheat -p 13725
```

Then will get the output about all global variables about this program
```
[205.405µs] Time of `parse elf`
[497.685µs] Time of `filter_symbol`
Matched count: 3
index: var_name                                 | var_size(B)
    0: sc_sig_arr                               |      60
    1: structure                                |       8
    2: techs                                    |      21
Please input index to choose the var(default is 0):
```

Input `2` and `Enter`, you will see the byte value and ascii content of this variable (control char that unvisible show as `.`)

```
0x0000: 0273 7472 2e77 6120 3a20 3f20 210a 6461 ┃ .str.wa : ? !.da
0x0010: 770d 3231 00                            ┃ w.21.
```

You also can specify the total name or partly keyword of the variable with option `-k`

```sh
sudo rcheat -p 3754914 -k sig_arr
```
```
...

0x0000: 2000 0000 0000 4855 5000 0000 494e 5400 ┃  .....HUP...INT.
0x0010: 0000 5155 4954 0000 494c 4c00 0000 5452 ┃ ..QUIT..ILL...TR
0x0020: 4150 0000 494f 5400 0000 4255 5300 0000 ┃ AP..IOT...BUS...
0x0030: 4650 4500 0000 4b49 4c4c 0000           ┃ FPE...KILL..
```

##  3. <a name='Todo'></a>Todo

*The development plan of the project and the functions to be implemented*

- [ ] use log crate such as `log/env_logger` etc.
- [ ] regex replace String.contain
- [ ] parse .debug* section
- [ ] use config.toml to reduce some inputs
- [ ] use `lua` to customized output
- [ ] search pid by process name (like linux command: `pidof/pgrep`)
- [x] if match more than 1 entry name, ask for which one to select
- [x] demangle symbols
