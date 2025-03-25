#set page(width: 210mm, height: auto)
#set text(font: ("Monaspace Neon", "Noto Sans CJK SC"), lang: "zh")
#show raw: set text(font: ("Monaspace Neon", "Noto Sans CJK SC"))
#set heading(numbering: "1.1")
#show heading: set block(below: 1em)

#page(
  margin: 2cm,
  header: none,
  footer: none,
)[
  // 封面内容
  #align(center + horizon, [
    #text(size: 24pt, weight: "bold", "Rua 使用手册")
    #v(1em) // 垂直间距
    #text(size: 16pt, "v1.0")
    #v(2em) // 垂直间距
    #text(size: 16pt, "bzhao")
    #v(1em) // 垂直间距
    #text(size: 14pt, "2025-02-13")
  ])
]

#outline(indent: 2em)
#pagebreak()

= mkinfo

`rua mkinfo` 用于生成目标平台的构建指令。该指令包含了众多常用的make变量，用于方便地定制构建行为。例如：通过传入 `-6/--ipv6` 选项，令构建目标中包含 `ipv6` 字样；通过传入 `-w/--webui` 选项，将在指令中的 `NOTBUILDWEBUI` 变量设置为 `0`（带WebUI）。

== 用法

用户可通过 `rua mkinfo -h` 查看帮助信息：

#figure(
  image(
    ".assets/manual.mkinfohelp.png"
  )
)

== 示例

+ `rua mkinfo -6 A1000`: 生成 `A1000` 平台的构建指令，启用 IPv6 支持
  #figure(
    image(
      ".assets/manual.mkinfo6.png"
    )
  )
+ `rua mkinfo -w A1000`: 生成 `A1000` 平台的构建指令，启用 WebUI 支持
  #figure(
    image(
      ".assets/manual.mkinfow.png"
    )
  )
+ `rua mkinfo -6w A1000`: 生成 `A1000` 平台的构建指令，启用 IPv6 支持以及 WebUI 支持
  #figure(
    image(
      ".assets/manual.mkinfo6w.png"
    )
  )
+ `rua mkinfo -s s A1000`: 生成 `A1000` 平台的构建指令，上传到 10.200.6.10 服务器，即苏州服务器\
  #figure(
    image(
      ".assets/manual.mkinfos.png"
    )
  )
+ `rua mkinfo --format json A1000`: 生成 `A1000` 平台的构建指令，输出格式指定为 JSON 格式，适合脚本使用
  #figure(
    image(
      ".assets/manual.mkinfojson.png"
    )
  )
#pagebreak()

= compdb

`rua compdb` 包含了众多子命令，分别用于生成、删除和管理编译数据库。编译数据库的存在是为了给C/C++语言服务器（Language Server, LS），如 clangd，提供编译指示，从而使其能够在代码库中正确地跳转。

编译数据库包含了代码库中各个源文件的编译指令，有了该指令后，LS就知道了该翻译单元的头文件查找路径和各种宏定义。通常而言，编译数据库是分构建目标的，如 `a-dnv-ipv6` 对应有一个编译数据库，`a-dnv` 对应有另一个编译数据库。

== 用法

用户可通过 `rua compdb -h` 查看帮助信息:
#figure(
  image(
    ".assets/manual.compdbhelp.png"
  )
)

compdb 包含七个子命令，分别是 `gen`, `add`, `del`, `ls`, `use`, `name`, `remark`。下面将分别介绍这些子命令的用法和示例。

=== 生成编译数据库

```bash
rua compdb gen <构建路径> <构建目标>
```

- 构建路径: `products/ngfw_as` or `products/ngfw_ak` or ...
- 构建目标: `a-dnv` or `hygon` or ...

#figure(
  image(
    ".assets/manual.compdbgenhelp.png"
  )
)

=== 归档编译数据库

```bash
❯ rua compdb add -h
Add the currently used compilation database into store as a new generation

Usage: rua compdb add [OPTIONS] <TARGET>

Arguments:
  <TARGET>  Target for the compilation database

Options:
  -r, --revision <REVISION>
          Revision for compilation database (defaults to current repo revision)
  -f, --compilation-database <COMPILATION-DATABASE>
          Use this compilation database other than the default (compile_commands.json)
  -h, --help
          Print help
```

- 构建目标: `a-dnv` or `hygon`...

=== 删除编译数据库

```bash
❯ rua compdb del -h
Delete compilation database generation(s) from store

Usage: rua compdb del [OPTIONS] [GENERATION-ID]

Arguments:
  [GENERATION-ID]  Generation to delete

Options:
  -a, --all      Remove all generations
  -n, --new <N>  Remove N newest generations
  -o, --old <N>  Remove N oldest generations
  -h, --help     Print help
```

=== 列出编译数据库

```bash
❯ rua compdb ls -h
List all compilation database generations in store

Usage: rua compdb ls

Options:
  -h, --help  Print help
```

=== 选择编译数据库

```bash
❯ rua compdb use -h
Select a compilation database generation from store to use

Usage: rua compdb use <GENERATION>

Arguments:
  <GENERATION>  Compilation database generation id

Options:
  -h, --help  Print help
```

=== 命名编译数据库

```bash
❯ rua compdb name -h
Name a compilation database generation

Usage: rua compdb name <GENERATION> <NAME>

Arguments:
  <GENERATION>  The compilation database generation
  <NAME>        Name for the compilation database

Options:
  -h, --help  Print help
```

=== 备注编译数据库

```bash
❯ rua compdb remark -h
rua compdb remark <GENERATION-ID> <备注>
Remark a compilation database generation

Usage: rua compdb remark <GENERATION> <REMARK>

Arguments:
  <GENERATION>  The compilation database generation
  <REMARK>      Remark for the compilation database generation

Options:
  -h, --help  Print help
```

== 示例

+ `rua compdb add`:\
  将当前使用的编译数据库（compile_commands.json）添加到store中\
  #figure(
    image(".assets/changelog.0_22_0.compdb_add.png")
  )
+ `rua compdb del 2`:\
  删除store中的第二个编译数据库\
  #figure(
    image(".assets/manual.compdbdel2.png")
  )
+ `rua compdb del -o 3`:\
  删除store中最旧的3个编译数据库\
  #figure(
    image(".assets/manual.compdbdelo3.png")
  )
+ `rua compdb del -n 2`:\
  删除store中较新的两个编译数据库\
  #figure(
    image(".assets/changelog.0_22_0.compdb_del_recent_2.png")
  )
+ `rua compdb del -a`:\
  删除store中所有的编译数据库\
  #figure(
    image(".assets/manual.compdbdela.png")
  )
+ `rua compdb name 1 A1600-A`:\
  为store中的编译数据库 generation 1 添加一个名字\
   #figure(
    image(".assets/changelog.0_22_0.compdb_name.png")
   )
+ `rua compdb remark 1 "Compilation database generation for A1600-A"`:\
  为store中的编译数据库 generation 1 添加备注\
  #figure(
    image(".assets/changelog.0_22_0.compdb_remark.png")
  )

#pagebreak()

= showcc

显示某个文件的编译指令，依赖于编译数据库。

== 用法

用户可通过 `rua showcc -h` 查看帮助信息：

```bash
❯ rua showcc -h
Show all possible compile commands for filename (based on compilation database)

Usage: rua showcc [OPTIONS] <SOURCE-FILE>

Arguments:
  <SOURCE-FILE>  Source file name for which to fetch all the available compile commands

Options:
  -c, --compdb <COMPDB>  Compilation database (defaults to file "compile_commands.json" in the current directory)
  -h, --help             Print help
```

== 示例

+ 显示 flow_first.c 文件的编译指令:
  ```bash
  rua showcc flow_first.c
  ```
  #figure(
    image(".assets/manual.showcc_flow_first.png")
  )
+ 显示 virtual_wire.c 的编译指令:
  ```bash
  rua showcc virtual_wire.c
  ```
  #figure(
    image(".assets/manual.showcc_virtual_wire.png")
  )

#pagebreak()

= review

该子命令与 autoreview-cops 工具相似，参数更符合直觉。

== 用法

```bash
⬢ [podman] ❯ rua review -h
Start a new review request or refresh the existing one if review-id provided

Usage: rua review [OPTIONS] --bug <BUG> [FILE]...

Arguments:
  [FILE]...  Files to be reviewed

Options:
  -n, --bug <BUG>                      Bug id for this review request (required)
  -r, --review-id <REVIEW-ID>          Existing review id
  -d, --diff-file <DIFF-FILE>          Diff file to be used
  -u, --reviewers <REVIEWERS>          Reviewers
  -b, --branch <BRANCH>                Branch name for this commit
  -p, --repo <REPO>                    Repository name
  -s, --revision <REVISION>            Revision to be used
  -t, --template-file <TEMPLATE-FILE>  Use customized template file (please ensure it can run through svn commit hooks)
  -h, --help                           Print help
```

== 示例

// + `rua review -n `:\
//   生成一个 review 请求\
//   #figure(
//     image(".assets/manual.review.png")
//   )

#pagebreak()

= perfan

`rua perfan` 用于对 profiling text 进行指令地址映射。

== 用法

```bash
❯ rua perfan -h
Extensively map instructions to file locations (inline expanded)

Usage: rua perfan [OPTIONS] <FILE>

Arguments:
  <FILE>  File to process (perf annotate output)

Options:
  -o, --format <FORMAT>  Output format [default: table] [possible values: json, table]
  -e, --elf <ELF>        Binary files used for addresses resolving
  -h, --help             Print help
```

== 示例

+ 在MX_MAIN分支下，使用 rua perfan 命令解析 profiling 文本中属于 d-plane 的地址:
  #figure(
    image(".assets/changelog.0_25_0.origtext.png"),
    caption: [
      原始 A3600 profiling 文本
    ],
    numbering: none,
  )
  #figure(
    image(".assets/changelog.0_25_0.ruaperfan.png"),
    caption: [
      A3600 profiling 文本经 `rua perfan` 解析后
    ],
    numbering: none,
  )
+ 传入多个 elf 参数，解析多个二进制的地址:
  ```bash
  rua perfan -e ./bin/obj-emulator-a-dnv-ipv6-2.0/d-plane -e ./bin/obj-emulator-a-dnv-ipv6-2.0/netd A3600.profile.txt
  ```
  注意: 当传入非C语言（包括C++）生成的二进制中，符号是混淆过的，函数名可能比较奇怪，这是正常现象。结果中提供有文件名和行号，凭此信息能够准确地定位代码行。

== 输出格式

#figure(
  image(".assets/manual.ruaperfanoutput.png"),
  caption: [
    Rua perfan 输出格式解析
  ],
  numbering: none,
)

== 工具比较

代码库中 `tool/perf2func` 具有相同的功能，本工具的优势在于：
- 速度更快，相比于 `perf2func` 提速1500倍左右
- 输出格式更友好，对内联函数有较好的展开处理，方便用户定位代码行

#pagebreak()

= init

rua 可以在 bash/zsh/fish 等 shell 中自动补全。

== 用法

用户可以通过 `rua init -h` 查看帮助信息：

```bash
❯ rua init -h
Generate completion for the given shell

Usage: rua init <SHELL>

Arguments:
  <SHELL>  Shell type [possible values: bash, elvish, fish, powershell, zsh]

Options:
  -h, --help  Print help

Note:
  eval "$(rua init bash)"  # Append this line to ~/.bashrc
  eval "$(rua init zsh)"   # Append this line to ~/.zshrc
```

== 示例

+ 在 bash 中自动补全:
  ```bash
  eval "$(rua init bash)"
  ```
+ 在 zsh 中自动补全:
  ```bash
  eval "$(rua init zsh)"
  ```
+ 在 fish 中自动补全:
  ```bash
  eval (rua init fish)
  ```
+ 在 powershell 中自动补全:
  ```bash
  rua init powershell | Out-String | Invoke-Expression
  ```
+ 在 elvish 中自动补全:
  ```bash
  eval (rua init elvish)
  ```
