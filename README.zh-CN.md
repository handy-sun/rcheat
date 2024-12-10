
# **rcheat**


**简体中文| [English](./README.md)**<br>
[![build-test](https://github.com/handy-sun/rcheat/actions/workflows/build-test.yml/badge.svg)](https://github.com/handy-sun/rcheat/actions/workflows/build-test.yml)
![latest_release](https://img.shields.io/github/v/tag/handy-sun/rcheat?label=release)
[![Crates.io](https://img.shields.io/crates/v/rcheat.svg)](https://crates.io/crates/rcheat)
![Linux](https://img.shields.io/badge/-Linux-grey?logo=linux)

> *在另一个Linux运行进程中获取/修改简单变量的值*

**请注意，本项目仅用于学习和研究目的，作者不对使用本项目造成的任何法律后果负责。**

<!-- vscode-markdown-toc -->
* 1. [安装](#)
	* 1.1. [通过cargo](#cargo)
	* 1.2. [构建源代码](#-1)
		* 1.2.1. [依赖项](#-1)
		* 1.2.2. [构建](#-1)
* 2. [简单示例](#-1)
* 3. [Todo](#Todo)

<!-- vscode-markdown-toc-config
	numbering=true
	autoSave=true
	/vscode-markdown-toc-config -->
<!-- /vscode-markdown-toc -->


##  1. <a name=''></a>安装

###  1.1. <a name='cargo'></a>通过cargo

<details>
<summary>安装cargo的一些方法</summary>

- 可以使用[rustup](https://rust-lang.github.io/rustup/)获得（推荐）
- 使用 Linux 包管理（例如 apt、yum、dnf、pacman）
- 从 [forge.rust-lang.org](https://forge.rust-lang.org/infra/archive-stable-version-installers.html) 下载离线 tarball
</details>

只需运行以下命令即可安装

```sh
cargo install --force rcheat
```

这将在您的 `~/.cargo/bin` 中安装 Cargo-make。
确保将 `~/.cargo/bin` 目录添加到 `PATH` 变量中。
您将有一个可用的可执行文件：*`rcheat`*

###  1.2. <a name='-1'></a>构建源代码

<!-- <a name="dependency"></a> -->
####  1.2.1. <a name='-1'></a>依赖项

- [cargo](https://github.com/rust-lang/cargo/) >= 1.74
- [rustc](https://www.rust-lang.org/) >= 1.74

建议使用最新版本

####  1.2.2. <a name='-1'></a>构建

```shell
git clone https://github.com/handy-sun/rcheat.git
cd rcheat
cargo build
```

您将有一个可用的可执行文件：*`./target/debug/rcheat`*

**提示：**
如果 `crates.io` 下载速度太慢。使用镜像来加速（例如使用[rsproxy](https://rsproxy.cn)）。


<a name="simple-example"></a>
##  2. <a name='-1'></a>简单示例

例如，带有一些全局变量的 `C` 源文件 `onlyc.c`：

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

然后编译并运行它：
```sh
gcc onlyc.c -o onlyc && ./onlyc
```

获取 `onlyc` 的 pid（例如使用命令：`pidof`）并将 `rcheat` 与 `-p` 选项一起使用：
**注意：该程序必须以 root 权限运行！**

```sh
pidof onlyc
# output: 13725
sudo rcheat -p 13725
```

然后将得到有关该程序的所有全局变量的输出
```
...
Matched count: 3
Index: var_name                                 | var_size(B)
    0: sc_sig_arr                               |      60
    1: structure                                |       8
    2: techs                                    |      21
Please input index to choose the var(default is 0):
```

输入 `2` 和 `Enter`，您将看到该变量的字节值和 ascii 内容（不可见的控制字符显示为 `.`）

```
0x0000: 0273 7472 2e77 6120 3a20 3f20 210a 6461 ┃ .str.wa : ? !.da
0x0010: 770d 3231 00                            ┃ w.21.
```

您还可以使用选项 `-k` 指定变量的总名称或部分关键字

```sh
sudo rcheat -p 13725 -k sig_arr
```
```
...

0x0000: 2000 0000 0000 4855 5000 0000 494e 5400 ┃  .....HUP...INT.
0x0010: 0000 5155 4954 0000 494c 4c00 0000 5452 ┃ ..QUIT..ILL...TR
0x0020: 4150 0000 494f 5400 0000 4255 5300 0000 ┃ AP..IOT...BUS...
0x0030: 4650 4500 0000 4b49 4c4c 0000           ┃ FPE...KILL..
```

版本 `0.1.3` 之后，选项 `-n/--name` 可以通过进程名称查询 pid

```
sudo rcheat -n onlyc -k sig_arr
```

##  3. <a name='Todo'></a>Todo

*项目的发展规划及拟实现的功能*

- [ ] 解析 `.debug*` 部分
- [ ] 使用日志箱，例如 `log/env_logger` 等。
- [ ] 将数据写入tracee进程的内存
- [ ] 使用config.toml减少一些输入
- [x] 使用像 `table` 这样的库来格式化矩阵表数据
- [x] 使用 `lua` 自定义输出
- [x] 按进程名称搜索 pid（如 linux 命令：`pidof/pgrep`）
- [x] 正则表达式替换 String.contain
- [x] 如果匹配超过 1 个条目名称，则询问选择哪一个
- [x] 分解符号





















































