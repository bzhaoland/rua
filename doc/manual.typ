#set page(
  width: 210mm,
  height: auto,
  numbering: "1"
)
#set text(font: ("Monaspace Neon", "Noto Sans CJK SC"), lang: "zh")
#set heading(numbering: "1.1", offset: 0)
#show heading: set block(below: 1em)
#show raw: set text(font: ("Monaspace Neon", "Noto Sans CJK SC"))
#show raw: set block(
  fill: luma(240),
  width: 100%,
  inset: 3mm,
  radius: 1.5mm,
)

#page(
  margin: 2cm,
  header: none,
  footer: none,
)[
  // 封面内容
  #align(center + horizon, [
    #text(size: 24pt, weight: "bold", "Rua 使用手册")
    #v(1em) // 垂直间距
    #text(size: 16pt, "v1.0.0")
    #v(2em) // 垂直间距
    #text(size: 16pt, "bzhao")
    #v(1em) // 垂直间距
    #text(size: 14pt, "2025-03-31")
  ])
]

#outline(indent: 2em)
#pagebreak()

= mkinfo

`rua mkinfo` 用于生成目标平台的构建指令。该指令包含了众多常用的make变量，用于方便地定制构建行为。例如：通过传入 `-6/--ipv6` 选项，令构建目标中包含 `ipv6` 字样；通过传入 `-w/--webui` 选项，将在指令中的 `NOTBUILDWEBUI` 变量设置为 `0`（带WebUI）。

== 用法

用户可通过 `rua mkinfo -h` 查看帮助信息：

```bash
❯ rua mkinfo -h
Get all matched makeinfos for product

Usage: rua mkinfo [OPTIONS] <NAME-OR-TARGET>

Arguments:
  <NAME-OR-TARGET>  Product name such as A1000, or build target (with --target switch on) such as a-dnv, View as a product name by default. Regex is also supported when using as a product name, e.g. 'X\d+80'

Options:
  -4, --ipv4                         Build with only IPv4 enabled
  -6, --ipv6                         Build with IPv6 enabled
  -g, --coverage                     Run coverage
  -c, --coverity                     Run coverity
  -d, --debug                        Build in debug mode (default is release mode)
      --format <FORMAT>              Output format for makeinfos, defaults to list [default: list] [possible values: csv, json, list, tsv]
  -p, --password                     Build with shell password enabled
  -w, --webui                        Build with WebUI enabled
  -s, --image-server <IMAGE-SERVER>  Server to upload the output image to [possible values: b, s]
      --nostrip <BINARY>             Binaries that get out of strip processing
      --by-target                    Treat the positional arg as a build target other than a product name
  -h, --help                         Print help

Examples:
  rua mkinfo A1000      # Makeinfo for A1000 without extra features
  rua mkinfo -6 A1000   # Makeinfo for A1000 with IPv6 enabled
  rua mkinfo -6w 'X\d+' # Makeinfos for X-series products with IPv6 and WebUI enabled using regex pattern
  rua mkinfo --target a-dnv  # Makeinfos for a-dnv target
```

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
+ `rua mkinfo -ss A1000`: 生成 `A1000` 平台的构建指令，上传到 10.200.6.10 服务器，即苏州服务器
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

`rua compdb` 包含了众多子命令，分别用于生成、删除和管理编译数据库。

编译数据库能够为C/C++语言服务器（Language Server, LS）提供各编译单元的编译指令，如 include 路径、编译宏，使之能够理解每个文件的编译方式，从而在代码代码库中正确地行走。

编译数据库的格式为 JSON 格式，文件名为 `compile_commands.json`，存放在当前工作目录下。该文件是一个数组，每个元素对应一个编译单元的编译指令。

编译数据库的每个元素包含了以下几个字段：
- `directory`: 编译单元所在的目录
- `command`: 编译单元的编译指令
- `file`: 编译单元的文件路径

一般来说，编译数据库是分构建目标的，如 `a-dnv-ipv6` 对应有一个编译数据库，`a-dnv` 对应有另一个编译数据库。对于后者而言，IPV6宏所包裹的代码不会被解析，LS视之为无效代码。

Rua 工具为每条命令和参数添加了充分的注释，当存在疑惑时请使用 `-h` 或 `--help` 参数查看帮助信息。例如，工具顶层命令帮助信息可通过在 shell 下执行 `rua compdb -h` 查看:

```bash
❯ rua compdb -h
Manipulate compilation database

Usage: rua compdb <COMMAND>

Commands:
  gen     Generate a JSON compilation database (JCDB) for the given target [aliases: generate]
  add     Archive the currently used compilation database into store as a new generation [aliases: ark, archive]
  del     Delete compilation database generation(s) from store [aliases: delete, rm, remove]
  ls      List all compilation database generations in store [aliases: list]
  use     Select a compilation database generation from store to use
  name    Name a compilation database generation
  remark  Remark a compilation database generation
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

== 生成编译数据库(Gen)

```sh
❯ rua compdb gen -h
Generate a JSON compilation database (JCDB) for the given target

Usage: rua compdb gen [OPTIONS] <PATH> <TARGET>

Arguments:
  <PATH>    Path for the target where platform-specific makefiles reside, such as 'products/vfw'
  <TARGET>  Target to build, such as 'a-dnv'

Options:
  -D, --define <KEY=VAL>
          Define a variable which will be passed to the underlying make command
  -e, --engine <ENGINE>
          Engine for generating compilation database (defaults to built-in) [possible values: built-in, intercept-build, bear]
  -b, --bear-path <BEAR>
          Path to the bear binary (defaults to /devel/sw/bear/bin/bear)
  -i, --intercept-build-path <INTERCEPT-BUILD>
          Path to the intercept-build binary (defaults to /devel/sw/llvm/bin/intercept-build)
  -h, --help
          Print help (see more with '--help')

Examples:
  rua compdb gen products/ngfw_as a-dnv                    # For A1000/A2000...
  rua compdb gen products/ngfw_as a-dnv-ipv6               # For A1000/A2000... with IPv6 support
  rua compdb gen -e intercept-build products/ngfw_as a-dnv # For A1000/A2000... using intercept-build
  rua compdb gen . a-dnv                                   # For A1000/A2000... under submod dir
  rua compdb gen -e bear . a-dnv                           # For A1000/A2000... under submod dir using bear 
  run compdb gen -e intercept-build . a-dnv                # For A1000/A2000... under submod dir using intercept-build

Caution:
  Some files are modified while running in built-in mode which is the default
  and faster:
  1. When running under project root dir:
     - scripts/last-rules.mk
     - scripts/rules.mk
     - Makefile
  2. When running under submod dir:
     - scripts/last-rules.mk
     - scripts/rules.mk
  These files may be left dirty if compdb process aborted unexpectedly. You
  could manually restore them by execute:
  svn revert Makefile scripts/last-rules.mk scripts/rules.mk
```

*参数解析：*

- `<PATH>`: 构建路径，例如 A1000/A2000平台的构建路径为 `products/ngfw_as`，K6580平台的构建路径为 `products/ngfw_ak` or ...
- `<TARGET>`: 构建目标，例如 A1000/A2000等平台的编译目标为 `a-dnv`，K6580平台的编译目标位 `hygon`，X8180平台的编译目标位 `tai`
- `-e/--engine`: 可选，引擎。候选值为 `built-in`、`intercept-build`、`bear`，默认值为 `built-in`
- `-D/--define`: 可选，变量定义。将会传递给底层的 make 命令
- `-b/--bear-path`: 可选，指定 bear 的路径，默认值为 `/devel/sw/bear/bin/bear`
- `-i/--intercept-build-path`: 可选，指定 intercept-build 的路径，默认值为 `/devel/sw/llvm/bin/intercept-build`

=== 使用默认方法(built-in)生成编译数据库

+ 在工程根目录下生成一个编译目标为 `kunlun-ipv6` 的编译数据库:
  #figure(
    image(
      ".assets/manual.compdbgen.png"
    )
  )
+ 在工程根目录下为X8180平台生成一个编译数据库，编译目标为 `tai`:
  #figure(
    image(
      ".assets/manual.compdbgentai.png"
    )
  )

=== 使用 intercept-build 方法生成编译数据库

intercept-build 是 LLVM 工具集合所提供的一个工具，能够拦截编译命令并生成编译数据库。

由于该工具的工作原理为拦截每次触发编译器编译的指令，因此不适合增量编译场景。增量编译场景下，编译器只会编译那些有变更的文件，而不会编译所有文件。这样一来，intercept-build 就无法捕获到所有的编译指令。
因此，建议在全量编译时使用该工具。

=== FQA
    
+ `rua compdb gen` 执行失败怎么办？\
  命令执行失败的原因有以下几个:
  - 无执行权限，建议使用 `chmod +x <RUA-PATH>` 添加可执行权限
  - 执行目录不正确，建议在工程根目录下执行
  - Makefile 状态不正确，简易检查 "scripts/rules.mk"、"scripts/last-rules.mk"、"Makefile" 三个文件是否被修改过。若有修改，建议执行 `svn revert` 恢复
  - 磁盘满了，建议检查磁盘空间是否足够。该类错误不但会导致编译数据库生成失败，还可能造成 "scripts/rules.mk"、"scripts/last-rules.mk"、"Makefile" 三个文件被修改
+ 无法生成第三方库的编译数据库怎么办？\
  `rua compdb gen` 默认使用 `built-in` 方法，该方法只生成我们的内部代码，暨通过 "scripts/last-rules.mk" 中一条用于编译目的的 recipe 来编译的文件。\
  若想单独生成第三方库的编译数据库，可使用 `bear` 或 `intercept-build` 方法。\
  若想使用 `bear` 或 `intercept-build` 方法生成整个工程的编译数据库，须在一个clean过的的分支下进行（从而进行全量编译），或者编译一个当前未被 ccache 缓存其编译信息的编译目标，否则会失败。

== 归档编译数据库(Add)

归档功能适合将其他工具生成的编译数据库归档到 store 中，以便管理。其用法如下：

```bash
❯ rua compdb add --help
Archive the currently used compilation database into store as a new generation

Usage: rua compdb add [OPTIONS] <TARGET>

Arguments:
  <TARGET>  Target specified for the compilation database

Options:
  -r, --revision <REVISION>
          Revision for compilation database (defaults to current repo revision)
  -f, --compilation-database <COMPILATION-DATABASE>
          Use this compilation database other than the default (compile_commands.json)
  -h, --help
          Print help

Examples:
    rua compdb add hygon-ipv6 # Archive compilation database for hygon-ipv6
    rua compdb add --revision 307164 hygon # Archive compilation database for hygon with a revision provided
```

*参数解析：*

- `<TARGET>`: 必填，用于指示编译数据库所对应的构建目标，如 `a-dnv`、`hygon`
- `-r/--revision`: 可选，用于指示编译数据库所对应的代码版本，默认使用当前工作目录的代码版本
- `-f/--compilation-database`：可选，用于手动指定编译数据库路径，默认使用当前工作目录下的 `compile_commands.json`

*例如：*

将当前工作目录下的 compile_commands.json 添加到 store 中:

#figure(
  image(
    ".assets/changelog.0_22_0.compdb_add.png"
  )
)

== 删除编译数据库(Del)

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

*例如：*

+ 删除store中的 Generation 2:
  #figure(
    image(
      ".assets/manual.compdbdel2.png"
    )
  )
+ 删除store中最旧的3个编译数据库:
  #figure(
    image(".assets/manual.compdbdelo3.png")
  )
+ 删除store中较新的两个编译数据库:
  #figure(
    image(".assets/changelog.0_22_0.compdb_del_recent_2.png")
  )
+ 删除store中所有的编译数据库:
  #figure(
    image(".assets/manual.compdbdela.png")
  )

== 列出编译数据库(Ls)

列出当前工作目录下所有的编译数据库。

```bash
❯ rua compdb ls -h
List all compilation database generations in store

Usage: rua compdb ls

Options:
  -h, --help  Print help
```

每个表项都有一个唯一的 `Generation ID`，且关联3个重要属性和2个可选属性：

#block(
  fill: rgb("#e2d6b94b"),
  width: 100%,
  inset: 3mm,
  radius: 1.5mm,
  [
    - `Revision`: 代码版本
    - `Target`: 构建目标
    - `Date`: 生成日期
    - `Name`: 可选，编译数据库的名字
    - `Remark`: 可选，编译数据库的备注
  ]
)

*例如：*

#figure(
  image(
    ".assets/manual.rua-compdb-ls.png"
  )
)

== 选择编译数据库(Use)

选中的编译数据库将会覆盖当前工作目录下的 compile_commands.json 文件。

```bash
❯ rua compdb use -h
Select a compilation database generation from store to use

Usage: rua compdb use <GENERATION>

Arguments:
  <GENERATION>  Compilation database generation id

Options:
  -h, --help  Print help
```

== 命名编译数据库(Name)

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

*例如：*

为store中的编译数据库 generation 1 添加一个名字:
#figure(
  image(".assets/changelog.0_22_0.compdb_name.png")
)

== 备注编译数据库(Remark)

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

*例如：*

为store中的编译数据库 Generation 1 添加备注\
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
  #figure(
    image(".assets/manual.showcc_flow_first.png")
  )
+ 显示 virtual_wire.c 的编译指令:
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

#pagebreak()

= perfan

`rua perfan` 用于对 profiling text 进行指令地址映射。

== 用法

```bash
Extensively map instructions to file locations (inline expanded)

Usage: rua perfan [OPTIONS] <FILE>

Arguments:
  <FILE>  Profiling text generated by perfan

Options:
  -o, --format <FORMAT>  Output format [default: table] [possible values: json, table]
  -e, --elf <ELF>        Binary files used for addresses resolving [aliases: exe, executable]
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
