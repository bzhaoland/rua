#set page(width: 210mm, height: auto)
#set text(font: ("Monaspace Neon", "Noto Sans CJK SC"), lang: "zh")
#show raw: set text(font: ("Monaspace Neon", "Noto Sans CJK SC"))
#set heading(numbering: "1.1")

#page(
  height: auto,
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
rua compdb add <构建目标>
```

- 构建目标: `a-dnv` or `hygon`...

#figure(
  image(
    ".assets/manual.compdbaddhelp.png"
  )
)

=== 删除编译数据库

```bash
rua compdb del [OPTIONS] [GENERATION-ID]
```

#figure(
  image(".assets/manual.compdbdelhelp.png")
)

=== 列出编译数据库

```bash
rua compdb ls
```

#figure(
  image(".assets/manual.compdblshelp.png")
)

=== 选择编译数据库

```bash
rua compdb use <GENERATION-ID>
```

#figure(
  image(".assets/manual.compdbusehelp.png")
)

=== 命名编译数据库

```bash
rua compdb name <GENERATION-ID> <名字>
```

#figure(
  image(".assets/manual.compdbnamehelp.png")
)

=== 备注编译数据库

```bash
rua compdb remark <GENERATION-ID> <备注>
```

#figure(
  image(".assets/manual.compdbremarkhelp.png")
)

== 示例

+ `rua compdb add`:\
  将当前使用的编译数据库（compile_commands.json）添加到store中\
  #figure(
    image(".assets/changelog.0_22_0.compdb_add.png")
  )
+ `rua compdb del --new 2`:\
  删除store中较新的两个编译数据库\
  #figure(
    image(".assets/changelog.0_22_0.compdb_del_recent_2.png")
  )
+ `rua compdb name 1 A1600-A`:\
  为store中的编译数据库 generation 1 添加一个名字，默认没有名字\
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

#figure(
  image(".assets/manual.showcchelp.png")
)

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

== 用法

== 示例

#pagebreak()

= perfan

== 用法

== 示例

#pagebreak()

= init

== 用法

== 示例

