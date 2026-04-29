
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
* 3. [Lua 脚本](#3-lua-脚本)
* 4. [Todo](#Todo)

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

##  3. Lua 脚本

自 `0.2.0` 版本起，rcheat 支持使用 Lua 脚本自定义二进制结构体解析和格式化表格输出。使用 `-f lua` 选项启用。

### 工作流程

1. 将 Lua 脚本文件放置在 `/etc/rcheat/lua/` 目录下
2. 使用 `-f lua` 运行 `rcheat`：

```sh
sudo rcheat -n onlyc -k structure -f lua
```

3. rcheat 加载内建的 `core.lua`，然后加载脚本目录下的所有 `.lua` 文件
4. 将变量名与 `Structure.match_table` 匹配，找到对应的别名
5. 调用 `Structure:new_<alias>(bytes)` 将原始字节解析为表格
6. 输出格式化的表格

### 编写 Lua 脚本

每个脚本必须定义一个全局 `Structure` 表，包含：

- `match_table` — 将变量名模式（Lua string.find）映射到别名
- `new_<alias>(bytes)` — 构造函数，解析原始字节并返回实例

列定义格式：

| 字段 | 说明 | 示例 |
|------|------|------|
| `name` | 列标题名 | `'id'`, `'health'` |
| `size` | 字节数 | `1`, `2`, `4`, `8` |
| `fmt` | [string.unpack](https://www.lua.org/manual/5.4/manual.html#6.4.2) 格式 | `'i'` 有符号, `'I'` 无符号, `'f'` 浮点, `'s'` 字符串, `'c'` 字符, `nil` 自动有符号整数 |

当 `fmt` 为 `'i'`、`'I'`、`'s'` 或 `'c'` 时，size 会自动附加（如 `i4`、`I1`）。当 `fmt` 为 `nil` 时，默认为 `i<size>`（有符号整数）。对于 `'f'`，大小由格式本身决定（`f` 为 4 字节，`d` 为 8 字节）。

### 示例

`/etc/rcheat/lua/example.lua`：

```lua
Structure = {}
Structure.__index = Structure

-- 匹配变量名中包含 'pcmStateList' 的变量，别名为 'psl'
Structure.match_table = {
    ['pcmStateList'] = 'psl',
}

-- 构造函数：将字节解析为包含 {id, stared, act} 列的表格
function Structure:new_psl(bytes)
    self.psl_col = {
        { name = 'id',     size = 4, fmt = 'i' },  -- 有符号 32 位整数
        { name = 'stared', size = 1, fmt = 'I' },  -- 无符号 8 位整数
        { name = 'act',    size = 4, fmt = 'f' },  -- 32 位浮点数
    }

    return setmetatable({ psl = SetupTableData(bytes, self.psl_col) }, Structure)
end
```

输出（rcheat 会将其格式化为对齐的表格）：

```
╭─────┬────┬────────╮
│ (i) │ id │ stared │   act │
├─────┼────┼────────┤───────┤
│   0 │  1 │      0 │  3.50 │
│   1 │  2 │      1 │  7.25 │
╰─────┴────┴────────┴───────╯
```

### 内建函数 (core.lua)

`SetupTableData(bytes, tab_list)` — 根据列定义遍历原始字节，返回二维表格。每行是一个 `{ name, size, data }` 条目数组。函数按每列的 `size` 切割字节数组，并使用指定的 `fmt` 通过 `string.unpack` 解包数据。

##  4. <a name='Todo'></a>Todo

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

