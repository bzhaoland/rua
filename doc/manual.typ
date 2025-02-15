#set page(height: auto)
#set text(font: ("Monaspace Neon", "Noto Sans CJK SC"), lang: "zh")
#show raw: set text(font: "Monaspace Neon")
#set heading(numbering: "1.1")

#page(
  width: 210mm,
  height: 100mm,
  margin: 2cm,
  header: none,
  footer: none,
)[
  // 封面内容
  #align(center + horizon, [
    #text(size: 24pt, weight: "bold", "Rua 使用手册")
    #v(2em) // 垂直间距
    #text(size: 16pt, "bzhao")
    #v(1em) // 垂直间距
    #text(size: 14pt, "2025-02-13")
  ])
]

#outline(indent: 2em)
#pagebreak()

= mkinfo

== 介绍

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
+ `rua mkinfo -w A1000`: 生成 `A1000` 平台的构建指令，启用 WebUI 支持
+ `rua mkinfo -s s A1000`: 生成 `A1000` 平台的构建指令，上传到 10.200.6.10 服务器，即苏州服务器
+ `rua mkinfo --format json A1000`: 生成 `A1000` 平台的构建指令，输出格式指定为 JSON 格式，适合脚本使用

= compdb

== 介绍

== 用法

== 示例

= showcc

== 介绍

== 用法

== 示例

= review

== 介绍

== 用法

== 示例

= perfan

== 介绍

== 用法

== 示例

= init

== 介绍

== 用法

== 示例

