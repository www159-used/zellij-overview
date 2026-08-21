# overview

[English](../../README.md) · 中文

在浮动窗口里查看并切换 Zellij 的 session 和 tab。

按 `Ctrl+y` 打开，用 `hjkl` 移动。session 钉在同一块板最前面，后面是当前 session 的 tab。在 session 上按 `e` 或 Flash tip 会进入该 session 的 tab 板；再按一次才真正跳走。当前 session 的 tab 在首页就能直接跳。项多时按 `s` 搜索当前板上的标题。

界面会按标题长度和浮窗尺寸，在有框卡片、无框紧凑和单列滚动之间切换。

<video src="../../asset/open.mov" controls width="720"></video>

## 安装

需要 Zellij 0.44 或更高版本。

### 使用 Release

从 [Releases](https://github.com/www159-used/zellij-overview/releases) 下载 WASM，重命名后放到插件目录：

```bash
mkdir -p ~/.config/zellij/plugins
mv overview-v*.wasm ~/.config/zellij/plugins/overview.wasm
```

### 从源码构建

仓库通过 `rust-toolchain.toml` 固定 Rust `1.95.0` 和 `wasm32-wasip1` target：

```bash
git clone https://github.com/www159-used/zellij-overview.git
cd zellij-overview
./scripts/install.sh
```

脚本会构建 WASM 并拷到 `~/.config/zellij/plugins/overview.wasm`。目标路径可用 `OVERVIEW_PLUGIN_PATH` 覆盖。

## 配置快捷键

把下面的配置加入 `~/.config/zellij/config.kdl` 的 `keybinds`。
直接使用 `file:` URL，不要通过插件 alias 启动，以保证实例识别一致。

```kdl
shared {
    bind "Ctrl y" {
        LaunchPlugin "file:~/.config/zellij/plugins/overview.wasm" {
            floating true
            skip_plugin_cache true
        }
    }
}
```

第一次打开时，Zellij 会询问读取和更改会话状态的权限，选择允许即可。再按一次 `Ctrl+y` 会关闭 overview。

## 使用说明

底部只列出常用键。按 `?` 可在 overview 里看完整帮助。

### 选择并跳转

1. 按 `Ctrl+y` 打开 overview。
2. 用 `h` `j` `k` `l` 或方向键移动选中框。
3. 按 `e` 或 `Enter`：在 session 上进入它的 tab 板，在 tab 上跳过去。

撞到列表两端会停住，不会绕回另一头。到头或到尾用 `gg` / `G`。滚动列表里，光标出窗时镜头会跟上。

### Flash 搜索

<video src="../../asset/flash.mov" controls width="720"></video>

假设有 `notes`、`feature/geo-db` 和 `logs` 三个 tab：

1. 按 `s` 进入搜索，footer 变成 `FLASH`。
2. 输入 `geo`。
3. 匹配项会高亮，并带上类似 `a` 的 tip。
4. 按 `a` 立刻跳到 `feature/geo-db`，不用再按 Enter。

`Backspace` 删查询。`Esc` 退出搜索，回到普通模式。

### session 和 tab 在同一块板

活着的 session 钉在最前面，后面是当前 session 的 tab。没有层 leader，也不再列出 pane。

- session 卡片以 `◆` 开头，并显示该 session 的 tab 数；够宽时用冷色框。
- tab 卡片仍是原来的标题和标记。
- 在 session 上按 `e` 或 Flash tip 会进入该 session 的 tab 板，不立刻切走。再在 tab 上按 `e` / tip：当前 session 直接跳，别人的一次落到那个 tab。
- 别人的 tab 板上没有其它 session 卡；`Esc` / `q` 先回首页。
- footer 显示当前 session 名；钻进某 session 后显示该 session 名。

够宽时全部是有框卡片并换行。不够宽时整页收成一列滚动：session 仍标 `◆`，tab 缩进一格。

### 返回上一个 tab

<video src="../../asset/previous.mov" controls width="720"></video>

标有 `[-]` 的卡片是按 `-` 将返回的位置。同 session 是上一个 tab。跳走过 session 之后，首页的 `[-]` 在离开的那个 session 上，`-` 先进入它的 tab 板；板上的 `[-]` 才是该 session 上次的 tab，再 `-` 一次落到那里。若之后在当前 session 里又跳过 tab，离开的 session 不再占 `-`，回到上一个 tab。

### 状态标记

- 紫色边框或 `›`：键盘当前选中的项
- `●`：当前实际所在的 session 或 tab
- `◆`：session 卡片
- `[-]`：按 `-` 将返回的 tab 或 session

### 自适应布局

<video src="../../asset/scroll.mov" controls width="720"></video>

- 空间充足时，卡片宽度跟随标题并自动换行
- 同一行可以有不同宽度，但卡片等高
- 空间较小时去掉边框和 padding
- 无法同时看清全部项时，改成单列滚动；右侧 `↑` / `↓` 表示还有未显示的项
- `hjkl` 移动选中项，镜头跟着走，两端停住
- 调整终端或浮窗尺寸后，布局会重新换行
- 上下移动会选邻行里横向位置最近的卡片

### 快捷键

| 按键 | 操作 |
| --- | --- |
| `h` `j` `k` `l` | 移动选中项（普通模式） |
| 方向键 | 移动选中项（普通模式和搜索模式） |
| `Ctrl+d` / `Ctrl+u` | 向下 / 向上半页（普通模式） |
| `Ctrl+f` / `Ctrl+b` | 向下 / 向上一页（普通模式） |
| `gg` / `G` | 跳到第一项 / 最后一项 |
| `zt` / `zz` / `zb` | 滚动列表里把当前项对齐顶部 / 居中 / 底部 |
| `PageDown` / `PageUp` | 向下 / 向上一页 |
| `e` / `Enter` | 打开选中 session 的 tab 板，或跳到选中 tab（普通模式） |
| `s` | 进入 Flash 搜索 |
| 任意字符 | 输入查询或 tip（搜索模式） |
| `Backspace` | 删除搜索输入（搜索模式） |
| `Esc` | 取消搜索；从 session 的 tab 板回首页；再按关闭 overview |
| `-` | 同 session 回上一个 tab；跨 session 先进入该 session 的 tab 板，再按跳到上次的 tab |
| `q` / `Esc` | 从 session 的 tab 板回首页，或关闭 overview |
| `?` | 打开或关闭完整帮助（普通模式；搜索模式下作为输入） |
| `Ctrl+y` | 再次按下时关闭 overview |

## 已知限制

Zellij 会整体显示或隐藏 floating layer。打开 overview 时，原本隐藏的其他 floating pane 也会暂时出现；关闭 overview 后，该图层会按打开前的状态恢复。目前插件 API 无法只显示一个可交互的 floating pane。

## 开发

```bash
cargo fmt --check
cargo lint
# 本机核心测试，不要使用 WASM target
cargo test --lib
cargo wasm
zellij -l zellij.kdl
```

开发布局从 `target/wasm32-wasip1/release/overview.wasm` 启动插件。界面由 Ratatui Buffer 渲染，Zellij 适配层负责读 tab / session / pane 并执行跳转。

关掉 overview 时会在插件 `/cache/usage.jsonl` 追加一条本机记录（键次数、Flash / hjkl / `-` / 是否钻进 session 板、结局、是否跨 session）。不记标题，不上传，最多 400 条。macOS 上这份文件在 `~/Library/Caches/org.Zellij-Contributors.Zellij`，Linux 在 `~/.cache/zellij`。之后用 `./scripts/usage-summary.sh` 汇总。

## 发布

安装 [`cargo-release`](https://github.com/crate-ci/cargo-release)，先预演再发布：

```bash
cargo install cargo-release
cargo release patch
cargo release patch --execute
```

也可将 `patch` 换成 `minor`、`major` 或明确版本号。

推送 `v*` tag 后，GitHub Actions 会验证项目、构建 WASM、生成 SHA-256 并创建 GitHub Release。

## 许可证

本项目采用 [GNU Affero General Public License v3.0 only](../../LICENSE)。
