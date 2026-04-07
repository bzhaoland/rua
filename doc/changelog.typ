#set page(width: 210mm, height: auto)
#set text(font: ("Monaspace Neon", "Source Han Sans SC"), lang: "zh")
#set heading(numbering: "1.1", offset: 0)
#show heading: set block(below: 1em)
#show raw: set text(font: ("Monaspace Neon", "Source Han Sans SC"))
#show raw: set block(
  fill: luma(240),
  width: 100%,
  inset: 3mm,
  radius: 1.5mm,
)

#page(
  margin: 2cm,
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


#let ftp_server_bj = "10.100.6.10"
#let ftp_server_sz = "10.200.6.10"

#let install_guide(version) = [
若已安装 rua 版本大于等于 #version，则可使用更新命令更新到最新版本：

```bash
rua update
```

否则，对于北京和苏州同学，分别执行以下步骤：

+ 北京，SSH登录到 *buildserver* 上，执行命令：\
  #raw(lang: "bash", block: true,
  "curl -LO ftp://10.100.6.10/bzhao/rua/" + version + "/rua  # 下载到本地\n"
  + "install -D rua ~/.local/bin/rua  # 安装到指定位置\n"
  + "rm -f rua  # 从当前目录删除"
  )
+ 苏州，SSH登录到 *buildserver* 上，执行命令：\
  #raw(lang: "bash", block: true,
  "curl -LO ftp://10.200.6.10/bzhao/rua/" + version + "/rua  # 下载到本地\n"
  + "install -D rua ~/.local/bin/rua  # 安装到指定位置\n"
  + "rm -f rua  # 从当前目录删除"
  )
]

= rua v2.0.1

#let rua_ver = "2.0.1"
#let rua_path = [bzhao/rua/#rua_ver/rua]

== 存放位置

- 北京: #ftp_server_bj/#rua_path
- 苏州: #ftp_server_sz/#rua_path

== 下载安装

#install_guide(rua_ver)

== 功能变更

+ 修复了一个问题，该问题导致 git 库里名字包含中文的文件无法被 clean 子命令正确删除（当 git 的 core.quotepath 为 true 时）。

#pagebreak()

= rua v2.0.0

#let rua_ver = "2.0.0"
#let rua_path = [bzhao/rua/#rua_ver/rua]

== 存放位置

- 北京: #ftp_server_bj/#rua_path
- 苏州: #ftp_server_sz/#rua_path

== 下载安装

#install_guide(rua_ver)

== 功能变更

+ 对 git 仓库进行了适配，几乎所有用法和之前相同；
+ [破坏性改动] compdb store schema 变动，移除了冗余的 Name 字段，现只有 Remark 字段可供添加注释。使用时会使用新的数据库文件存储生成过的 compile_commands.json

#pagebreak()

= rua v1.4.0

#let rua_ver = "1.4.0"
#let rua_path = [bzhao/rua/#rua_ver/rua]

== 存放位置

- 北京: #ftp_server_bj/#rua_path
- 苏州: #ftp_server_sz/#rua_path

== 下载安装

北京，SSH登录到 *buildserver* 上，执行命令：

```bash
curl -LO ftp://10.100.6.10/bzhao/rua/1.4.0/rua  # 下载到本地
install -D rua ~/.local/bin/rua  # 安装到指定位置
rm -f rua  # 从当前目录删除
```

苏州，SSH登录到 *buildserver* 上，执行命令：

```bash
curl -LO ftp://10.200.6.10/bzhao/rua/1.4.0/rua  # 下载到本地
install -D rua ~/.local/bin/rua  # 安装到指定位置
rm -f rua  # 从当前目录删除
```

== 功能变更

+ `rua update`: 本版本新增了update命令，后续可通过在命令命令行执行 `rua update` 来将 rua 更新到最新版，简化操作步骤
+ `rua update --pin <VERSION>`: 可指定更新到特定版本

== 使用示例

+ `rua update`: 自动下载和安装最新版本的 rua\
  #figure(
    image("assets/changelog.1_4_0.update.png"),
    numbering: none,
  )
+ `rua update --pin 1.3.1`: 自动下载和安装指定版本的 rua (注意，\<1.4.0的版本不具备升级功能)\
  #figure(
    image("assets/changelog.1_4_0.update_pin.png"),
    numbering: none,
  )

#pagebreak()

= rua v1.3.1

#let rua_ver = "1.3.1"
#let rua_path = [bzhao/rua/#rua_ver/rua]

== 存放位置

- 北京: #ftp_server_bj/#rua_path
- 苏州: #ftp_server_sz/#rua_path

== 下载安装

北京，SSH登录到 *buildserver* 上，执行命令：

```bash
curl -LO ftp://10.100.6.10/bzhao/rua/1.3.1/rua  # 下载到本地
install -D rua ~/.local/bin/rua  # 安装到指定位置
rm -f rua  # 从当前目录删除
```

苏州，SSH登录到 *buildserver* 上，执行命令：

```bash
curl -LO ftp://10.200.6.10/bzhao/rua/1.3.1/rua  # 下载到本地
install -D rua ~/.local/bin/rua  # 安装到指定位置
rm -f rua  # 从当前目录删除
```

== 功能变更

+ `rua compdb gen`: 适配 MX_MAIN revision 312960 的Makefile改动

#pagebreak()

= rua v1.3.0

#let rua_ver = "1.3.0"
#let rua_path = [bzhao/rua/#rua_ver/rua]

== 存放位置

- 北京: #ftp_server_bj/#rua_path
- 苏州: #ftp_server_sz/#rua_path

== 下载安装

北京，SSH登录到 *buildserver* 上，执行命令：

```bash
curl -LO ftp://10.100.6.10/bzhao/rua/1.3.0/rua  # 下载到本地
install -D rua ~/.local/bin/rua  # 安装到指定位置
rm -f rua  # 从当前目录删除
```

苏州，SSH登录到 *buildserver* 上，执行命令：

```bash
curl -LO ftp://10.200.6.10/bzhao/rua/1.3.0/rua  # 下载到本地
install -D rua ~/.local/bin/rua  # 安装到指定位置
rm -f rua  # 从当前目录删除
```

== 功能变更

+ `rua compdb`: 新增命令 merge，可将多个编译数据库手动个合入当前文件夹下的编译数据库
+ `rua compdb gen`: 新增选项 `--merge`，可在生成时指定要合入的其他便宜数据库，可通过指定多次该选项来传递多个编译数据库
+ `rua compdb gen`: 新增配置项 `merge`，列表类型，和 `rua compdb gen` 中的 `--merge` 效用相同

== 使用示例

+ `rua compdb merge --target a-dnv path/to/another/compile_commands.json`\
  会将 `path/to/another/compile_commands.json` 里面的内容提取出来，插入到当前目录下的 `compile_commands.json` 中。\
  为了指示最终的数据库针对哪个编译目标而生成（在 rua compdb ls 中显示），需要传入一个额外的 `--target` 参数
+ `rua compdb gen --merge path/to/another/compile_commands.json products/ngfw_as a-dnv`\
  除了像以往一样生成编译数据库外，还会合入 path/to/another/compile_commands.json
+ 将下述内容写入用户配置文件 `~/.config/rua/config.toml` 或工程根目录下的配置文件 `.rua/config.toml`，可实现自动合入（工程根目录下配置文件优先）
  ```toml
  [compdb]
  merge = ["path/to/another/compile_commands.json"]
  ```

新增命令和选项的用法可随时通过 `-h` 选项查看。

#pagebreak()

= rua v1.2.3

#let rua_ver = "1.2.3"
#let rua_path = [bzhao/rua/#rua_ver/rua]

== 存放位置

- 北京: #ftp_server_bj/#rua_path
- 苏州: #ftp_server_sz/#rua_path

== 下载安装

北京，SSH登录到 *buildserver* 上，执行命令：

```bash
curl -LO ftp://10.100.6.10/bzhao/rua/1.2.3/rua  # 下载到本地
install -D rua ~/.local/bin/rua  # 安装到指定位置
rm -f rua  # 从当前目录删除
```

苏州，SSH登录到 *buildserver* 上，执行命令：

```bash
curl -LO ftp://10.200.6.10/bzhao/rua/1.2.3/rua  # 下载到本地
install -D rua ~/.local/bin/rua  # 安装到指定位置
rm -f rua  # 从当前目录删除
```

== 功能变更

+ rua clean: 修复了删除 WebUI 文件夹时报错的问题

#pagebreak()

= rua v1.2.2

#let rua_ver = "1.2.2"
#let rua_path = [bzhao/rua/#rua_ver/rua]

== 存放位置

- 北京: #ftp_server_bj/#rua_path
- 苏州: #ftp_server_sz/#rua_path

== 下载安装

北京，SSH登录到 *buildserver* 上，执行命令：

```bash
curl -LO ftp://10.100.6.10/bzhao/rua/1.2.2/rua  # 下载到本地
install -D rua ~/.local/bin/rua  # 安装到指定位置
rm -f rua  # 从当前目录删除
```

苏州，SSH登录到 *buildserver* 上，执行命令：

```bash
curl -LO ftp://10.200.6.10/bzhao/rua/1.2.2/rua  # 下载到本地
install -D rua ~/.local/bin/rua  # 安装到指定位置
rm -f rua  # 从当前目录删除
```

== 功能修复

+ compdb: 适配最新MX_MAIN分支，修复了Makefile优化后编译数据库生成失败的问题
+ clean: 代码清理，功能类似现在的 MX_MAIN 分支上进行了 Makefile 优化后的 stoneos-clean target。此前存在一些问题，本次完善了该功能。clean 会做以下三个工作：
  + 删除 target 文件夹
  + 删除 webui 文件夹（与分支同名）
  + 删除未受SVN管控的文件，改功能与 make stoneos-clean 相似。
  Rua clean 允许保留特定文件（通过配置文件或命令行参数）。既可以通过传入 cli 参数，也可以通过配置文件来实现特定文件的保留。

== 使用示例

+ 清除工程根目录下的所有编译文件:
  ```bash
  rua clean .
  ```
  注意: 该命令会删除当前目录下的所有编译文件，包括 webui 文件夹和 target 文件夹
+ 清除工程根目录下的所有编译文件，但保留文件夹 .rua、.cache 和文件 .ignore 和 compile_commands.json:
  ```bash
  rua clean -n .rua -n .cache -n .ignore -n compile_commands.json .
  ```
  #figure(
    image("assets/changelog.1_2_2.clean.png"),
  )
+ 在 \~/.rua/config.toml 或 \$workspace/.rua/config.toml 中添加下面的配置内容，rua clean 在执行时会自动忽略这些文件:
  ```toml
  [clean]
  ignores = ["compile_commands.json", ".cache", ".rua", ".ignore"]
  ```
  #figure(
    image("assets/changelog.1_2_2.clean_config.png"),
  )

#pagebreak()

= rua v1.2.1

#let rua_ver = "1.2.1"
#let rua_path = [bzhao/rua/#rua_ver/rua]

== 存放位置

- 北京: #ftp_server_bj/#rua_path
- 苏州: #ftp_server_sz/#rua_path

== 下载安装

北京，SSH登录到 *buildserver* 上，执行命令：

```bash
curl -LO ftp://10.100.6.10/bzhao/rua/1.2.1/rua  # 下载到本地
install -D rua ~/.local/bin/rua  # 安装到指定位置
rm -f rua  # 从当前目录删除
```

苏州，SSH登录到 *buildserver* 上，执行命令：

```bash
curl -LO ftp://10.200.6.10/bzhao/rua/1.2.1/rua  # 下载到本地
install -D rua ~/.local/bin/rua  # 安装到指定位置
rm -f rua  # 从当前目录删除
```

== 功能修复

- review: 分支获取错误修复。当在非工程根目录下使用 rua review 时，上传到 review board 后显示分支错误。reported-by\@lnzeng
- compdb: 优化输出，消除了 hsdocker7 脚本所产生的冗余输出

#pagebreak()

= rua v1.2.0

#let rua_ver = "1.2.0"
#let rua_path = [bzhao/rua/#rua_ver/rua]

== 存放位置

- 北京: #ftp_server_bj/#rua_path
- 苏州: #ftp_server_sz/#rua_path

== 下载安装

北京，SSH登录到 *buildserver* 上，执行命令：

```bash
curl -LO ftp://10.100.6.10/bzhao/rua/1.2.0/rua  # 下载到本地
install -D rua ~/.local/bin/rua  # 安装到指定位置
rm -f rua  # 从当前目录删除
```

苏州，SSH登录到 *buildserver* 上，执行命令：

```bash
curl -LO ftp://10.200.6.10/bzhao/rua/1.2.0/rua  # 下载到本地
install -D rua ~/.local/bin/rua  # 安装到指定位置
rm -f rua  # 从当前目录删除
```

== 功能变更

+ mkinfo: 添加对 R6 分支的支持
+ compdb: 添加对 R4/R6 分支的支持

== 工具当前对各分支的支持情况

#let y = sym.checkmark
#let n = sym.crossmark
#let d = ""

#table(
  align: center,
  columns: 7,
  [子命令], [R4], [R6], [R8], [R10], [R11], [MX_MAIN],
  [mkinfo], [#n], [#y], [#y], [#y], [#y], [#y],
  [compdb], [#y], [#y], [#y], [#y], [#y], [#y],
  [perfan], table.cell(
    [
      不区分
    ],
    colspan: 6,
    align: center,
    fill: luma(200),
  ),
)

#pagebreak()

= rua v1.1.0

#let rua_ver = "1.1.0"
#let rua_path = [bzhao/rua/#rua_ver/rua]

== 存放位置

- 北京: #ftp_server_bj/#rua_path
- 苏州: #ftp_server_sz/#rua_path

== 下载安装

北京，SSH登录到 *buildserver* 上，执行命令：

```bash
curl -LO ftp://10.100.6.10/bzhao/rua/1.1.0/rua  # 下载到本地
install -D rua ~/.local/bin/rua  # 安装到指定位置
rm -f rua  # 从当前目录删除
```

苏州，SSH登录到 *buildserver* 上，执行命令：

```bash
curl -LO ftp://10.200.6.10/bzhao/rua/1.1.0/rua  # 下载到本地
install -D rua ~/.local/bin/rua  # 安装到指定位置
rm -f rua  # 从当前目录删除
```

== 功能增强（不影响现有功能）

- `rua compdb ls`: 添加当前使用项指示。下载新版本后，需要执行 `rua compdb use` 后指示才会显示出来，注意此操作会切换当前工作区下的编译数据库。
  #figure(
    image("assets/changelog.1_1_0.compdblsmigrate.png")
  )

#pagebreak()

= rua v1.0.0

#let ftp_server_bj = "10.100.6.10"
#let ftp_server_sz = "10.200.6.10"
#let rua_ver = "1.0.0"
#let rua_path = [bzhao/rua/#rua_ver/rua]

== 存放位置

- 北京: #ftp_server_bj/#rua_path
- 苏州: #ftp_server_sz/#rua_path

== 下载安装

北京，SSH登录到 *buildserver* 上，执行命令：

```bash
curl -LO ftp://10.100.6.10/bzhao/rua/1.0.0/rua  # 下载到本地
install -D rua ~/.local/bin/rua  # 安装到指定位置
rm -f rua  # 从当前目录删除
```

苏州，SSH登录到 *buildserver* 上，执行命令：

```bash
curl -LO ftp://10.200.6.10/bzhao/rua/1.0.0/rua  # 下载到本地
install -D rua ~/.local/bin/rua  # 安装到指定位置
rm -f rua  # 从当前目录删除
```

== 版本变更

- `rua perfan`: 简化参数使用
  - cli: 重命名参数 `-b/--binary` 为 `-e/--elf`
  - cli: 移除冗余参数 `-d/--daemon`，因为 daemon name 始终等于 elf 的文件名
  - cli: 现可通过传入多次 `-e/--elf` 参数来指定多个 elf 文件，同时解析多个二进制程序的地址
  - output: daemon summary percentage 百分比精确到小数点后四位

== 使用示例

+ 在MX_MAIN分支下，使用 rua perfan 命令解析 profiling 文本中属于 d-plane 的地址:
  #figure(
    image("assets/changelog.0_25_0.origtext.png"),
    caption: [
      原始 A3600 profiling 文本
    ],
    numbering: none,
  )
  #figure(
    image("assets/changelog.0_25_0.ruaperfan.png"),
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

#pagebreak()

= rua v0.25.0

#let ftp_server_bj = "10.100.6.10"
#let ftp_server_sz = "10.200.6.10"
#let rua_ver = "0.25.0"
#let rua_path = [bzhao/rua/#rua_ver/rua]

== 存放位置

- 北京: #ftp_server_bj/#rua_path
- 苏州: #ftp_server_sz/#rua_path

== 下载安装

北京，SSH登录到 *buildserver* 上，执行命令：

```bash
curl -LO ftp://10.100.6.10/bzhao/rua/0.25.0/rua  # 下载到本地
install -D rua ~/.local/bin/rua  # 安装到指定位置
rm -f rua  # 从当前目录删除
```

苏州，SSH登录到 *buildserver* 上，执行命令：

```bash
curl -LO ftp://10.200.6.10/bzhao/rua/0.25.0/rua  # 下载到本地
install -D rua ~/.local/bin/rua  # 安装到指定位置
rm -f rua  # 从当前目录删除
```

== 版本变更

- `rua perfan`: 简化参数使用
  - cli: 重命名参数 `-b/--binary` 为 `-e/--elf`
  - cli: 移除冗余参数 `-d/--daemon`，因为 daemon name 始终等于 elf 的文件名
  - cli: 现可通过传入多次 `-e/--elf` 参数来指定多个 elf 文件，同时解析多个二进制程序的地址
  - output: daemon summary percentage 百分比精确到小数点后四位

== 使用示例

+ 在MX_MAIN分支下，使用 rua perfan 命令解析 profiling 文本中属于 d-plane 的地址:
  #figure(
    image("assets/changelog.0_25_0.origtext.png"),
    caption: [
      原始 A3600 profiling 文本
    ],
    numbering: none,
  )
  #figure(
    image("assets/changelog.0_25_0.ruaperfan.png"),
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

#pagebreak()

= rua v0.24.0

== 存放位置

- 北京: 10.100.6.10/bzhao/rua/0.24.0/rua
- 苏州: 10.200.6.10/bzhao/rua/0.24.0/rua

== 下载安装

北京，SSH登录到 *buildserver* 上，执行命令：

```bash
curl -LO ftp://10.100.6.10/bzhao/rua/0.24.0/rua  # 下载到本地
install -D rua ~/.local/bin/rua  # 安装rua到指定位置
```

苏州，SSH登录到 *buildserver* 上，执行命令：

```bash
curl -LO ftp://10.200.6.10/bzhao/rua/0.24.0/rua  # 下载到本地
install -D rua ~/.local/bin/rua  # 安装rua到指定位置
```

== Changes

- `rua perfan`: perfan 输出格式趋稳，在 profiling 文本的解析速度上，相比于现有工具提速1500倍左右
  #figure(
    image("assets/changelog.0_24_0.ruaperf.png"),
    caption: [ rua perf 耗时 \<0.5s ]
  )
  #figure(
    image("assets/changelog.0_24_0.perf2func.png"),
    caption: [ 现有工具耗时 >10min ]
  )
  
#pagebreak()

= rua v0.23.1

- 北京: 10.100.6.10/bzhao/rua/0.23.1/rua
- 苏州: 10.200.6.10/bzhao/rua/0.23.1/rua

== BuildServer (北京) 上可通过命令下载和新增执行权限

```bash
curl -LO ftp://10.100.6.10/bzhao/rua/0.23.1/rua
chmod +x ./rua
```
== Changes

- `rua review`: fix branch name error when it consits of symbols which is not dash or in "word" character class

#pagebreak()

= rua v0.23.0

- 北京: 10.100.6.10/bzhao/rua/0.23.0/rua
- 苏州: 10.200.6.10/bzhao/rua/0.23.0/rua

== BuildServer (北京) 上可通过命令下载和新增执行权限

```bash
curl -LO ftp://10.100.6.10/bzhao/rua/0.23.0/rua
chmod +x ./rua
```

== Changes

- `rua mkinfo`: 现可借助 `--by-target` 参数，根据编译目标查找完整编译信息

== Examples

- `rua mkinfo --by-target zxc`:
  #figure(
    image("assets/changelog.0_23_0.mkinfobytarget.png"),
    caption: [
      根据编译目标`zxc`查找编译信息
    ],
    numbering: none,
  )
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
    image("assets/changelog.0_22_0.compdb_add.png"),
    caption: [
      添加编译数据库
    ],
    numbering: none,
  )
- `rua compdb name 1 A1600-A`:\
  为store中的编译数据库 generation 1 添加一个名字，默认没有名字\
   #figure(
    image("assets/changelog.0_22_0.compdb_name.png"),
    caption: [
      命名编译数据库
    ],
    numbering: none,
   )
- `rua compdb remark 1 "Compilation database generation for A1600-A"`:\
  为store中的编译数据库 generation 1 添加备注\
  #figure(
    image("assets/changelog.0_22_0.compdb_remark.png"),
    caption: [
      添加编译数据库备注
    ],
    numbering: none,
  )
- `rua compdb del --new 2`:\
  删除store中较新的两个编译数据库\
  #figure(
    image("assets/changelog.0_22_0.compdb_del_recent_2.png"),
    caption: [
      删除较新的两个编译数据库
    ],
    numbering: none,
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
