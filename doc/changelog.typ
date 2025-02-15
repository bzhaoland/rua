#set page(height: auto)
#set text(font: ("Monaspace Neon", "Noto Sans CJK SC"), lang: "zh")
#show raw: set text(font: ("Monaspace Neon", "Noto Sans CJK SC"))
#set heading(offset: 0)

#page(
  width: 210mm,
  height: 100mm,
  margin: 2cm,
  header: none,
  footer: none,
)[
  // 封面内容
  #align(center + horizon, [
    #text(size: 24pt, weight: "bold", "Changelog")
    #v(2em) // 垂直间距
    #text(size: 16pt, "bzhao")
    #v(1em) // 垂直间距
    #text(size: 14pt, "2025-02-05")
  ])
]

#outline(indent: 2em)
#pagebreak()

= rua v0.22.0

- 北京: 10.100.6.10/bzhao/rua/0.22.0/rua
- 苏州: 10.200.6.10/bzhao/rua/0.22.0/rua

== BuildServer (北京) 上可通过命令下载和新增执行权限

```bash
curl -LO ftp://10.100.6.10/bzhao/rua/0.22.0/rua
chmod +x ./rua
```

== Changes

+ 功能变更:
  - compdbs 的 table schema 变动，新增 name 和 remark 列
+ 功能新增:
  - `rua compdb add`: 将当前目录中正在使用的编译数据库加到store
  - `rua compdb name`: 为store中的某个编译数据库命名，名字要求必须唯一
  - `rua compdb remark`: 为store中的某个编译数据库添加备注
+ 功能增强:
  - `rua compdb del`: 新增 `--new/--old` 选项，用于删除较新或较旧的编译数据库

== Examples

- `rua compdb add`:\
  将当前使用的编译数据库（compile_commands.json）添加到store中\
  #figure(
    image(".assets/changelog.0_22_0.compdb_add.png"),
    caption: [
      添加编译数据库
    ]
  )
- `rua compdb name 1 A1600-A`:\
  为store中的编译数据库 generation 1 添加一个名字，默认没有名字\
   #figure(
    image(".assets/changelog.0_22_0.compdb_name.png"),
    caption: [
      命名编译数据库
    ]
   )
- `rua compdb remark 1 "Compilation database generation for A1600-A"`:\
  为store中的编译数据库 generation 1 添加备注\
  #figure(
    image(".assets/changelog.0_22_0.compdb_remark.png"),
    caption: [
      添加编译数据库备注
    ]
  )
- `rua compdb del --new 2`:\
  删除store中较新的两个编译数据库\
  #figure(
    image(".assets/changelog.0_22_0.compdb_del_recent_2.png"),
    caption: [
      删除较新的两个编译数据库
    ]
  )

== Notes

+ store schema 变动，需删除原 store (.rua/compdbs.db3) 后再运行该版本
#pagebreak()

= rua v0.21.1

== 存放位置

- 北京: 10.100.6.10/bzhao/rua/0.21.1/rua
- 苏州: 10.200.6.10/bzhao/rua/0.21.1/rua

== BuildServer (北京) 上可通过命令下载和新增执行权限

```bash
curl -LO ftp://10.100.6.10/bzhao/rua/0.21.1/rua
chmod +x ./rua
```
== Changes

+ 问题修复:
  - 修复了 `rua init` 在非svn库路径下执行时报错的问题

#pagebreak()

= rua v0.21.0

== 存放位置

- 北京: 10.100.6.10/bzhao/rua/0.21.0/rua
- 苏州: 10.200.6.10/bzhao/rua/0.21.0/rua

== BuildServer (北京) 上可通过命令下载和新增执行权限

```bash
curl -LO ftp://10.100.6.10/bzhao/rua/0.21.0/rua
chmod +x ./rua
```
== Changes

+ 功能变更:
  - `rua compdb <DIRECTORY> <TARGET>` 移除, 使用 `rua compdb gen <DIRECTORY> <TARGET>` 代替
+ 功能新增:
  - `rua compdb gen` 每次生成新的编译数据库时会将编译数据库同步存入 .rua/compdbs.db3  这一历史数据库中
  - `rua compdb ls`: 显示历来生成的编译数据库
  - `rua compdb use`: 指定的编译数据库(历史数据库中存储的)
  - `rua compdb rm`: 在历史数据库中移除指定或所有编译数据库

== Examples
  - `rua compdb gen products/ngfw_as a-dnv-ipv6`:\
    生成对应于A系列平台的编译数据库，同时将该数据库存入 .rua/compdbs.db3
  - `rua compdb ls`:\
    显示历史数据库中存储的编译数据库:
    ```txt
    Generation   Branch           Revision   Target       Date
    1            HAWAII_REL_R11   306454     a-dnv-ipv6   2025-02-08 12:28:38
    ```
  - `rua compdb use 2`:\
    使用历史数据库中的第二个编译数据库，该数据库会被解压到当前目录下，并替代当前目录下的编译数据库文件 compile_commands.json
  - `rua compdb rm 2`:\
    移除历史数据库中的第二个编译数据库

== Notes

+ 所有子命令后都可通过传入 `--help` 选项来查看相关帮助信息
+ 每次生成的数据库在存入后台历史数据库中时会自动压缩，35M大小的编译数据库在压缩后会变为300K左右，无需担心占用过多的存储空间
+ 历史数据库中的 generation id 严格递增，不会复用已删除的 generation id