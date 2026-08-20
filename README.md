# overview

在一个浮动窗口中查看并切换 Zellij tab。

按 `Ctrl+y` 打开 overview，选择目标后按 `e`；tab 较多时，按 `s` 输入标题片段，
再按候选项旁的提示字符即可直接跳转。

界面会根据标题长度和浮窗尺寸自动切换有框流式布局、无框紧凑布局和滚动列表。

## 安装

需要 Zellij 0.44 或更高版本。

### 使用 Release

从 [Releases](https://github.com/www159-used/zellij-overview/releases) 下载 WASM，
将它重命名并放到 Zellij 插件目录：

```bash
mkdir -p ~/.config/zellij/plugins
mv overview-v*.wasm ~/.config/zellij/plugins/overview.wasm
```

### 从源码构建

仓库已固定 Rust 工具链和 WASM target：

```bash
git clone https://github.com/www159-used/zellij-overview.git
cd zellij-overview
cargo wasm
mkdir -p ~/.config/zellij/plugins
cp target/wasm32-wasip1/release/overview.wasm ~/.config/zellij/plugins/
```

## 配置快捷键

将下面的配置加入 `~/.config/zellij/config.kdl` 的 `keybinds`：

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

第一次打开时，Zellij 会询问读取和更改会话状态的权限，选择允许即可。

## 使用

### 选择并跳转

1. 按 `Ctrl+y` 打开 overview。
2. 用 `h`、`j`、`k`、`l` 或方向键移动选中框。
3. 按 `e` 或 `Enter` 跳转。

### 快速搜索

假设存在 `notes`、`feature/geo-db` 和 `logs` 三个 tab：

1. 按 `s` 进入搜索。
2. 输入 `geo`。
3. 匹配项会高亮，并显示类似 `a` 的提示字符。
4. 按 `a` 立即跳到 `feature/geo-db`，无需再按 Enter。

### 返回上一个 tab

标有 `[-]` 的卡片是上一次聚焦的 tab。按 `-` 可直接返回。

### 状态标记

- 紫色边框或 `›`：键盘当前选中的 tab
- `●`：当前实际所在的 tab
- `[-]`：按 `-` 将返回的 tab

### 自适应布局

- 空间充足时，卡片宽度跟随标题长度并自动换行
- 同一行可以包含不同宽度的卡片，但所有卡片保持等高
- 空间较小时自动去掉边框和 padding
- 无法同时清晰显示全部 tab 时切换为单列滚动列表
- 方向键或普通模式下的 `hjkl` 会移动选中项，并自动滚动到可见位置
- 滚动列表右侧的 `↑` / `↓` 表示还有未显示的 tab

调整终端或浮窗尺寸后，布局会自动重新换行。上下移动会选择相邻行中横向位置最近的卡片。

### 快捷键

底部只显示常用操作；按 `?` 可在 overview 内查看完整快捷键帮助。

| 按键 | 操作 |
| --- | --- |
| `h` `j` `k` `l` | 移动选中项（普通模式） |
| 方向键 | 移动选中项（普通模式和搜索模式） |
| `Ctrl+d` / `Ctrl+u` | 向下 / 向上滚动半页 |
| `Ctrl+f` / `Ctrl+b` | 向下 / 向上滚动一页 |
| `gg` / `G` | 跳到第一个 / 最后一个 tab |
| `zt` / `zz` / `zb` | 将当前 tab 对齐顶部 / 居中 / 底部 |
| `PageDown` / `PageUp` | 向下 / 向上滚动一页 |
| `e` / `Enter` | 跳到选中的 tab（普通模式） |
| `s` | 进入搜索 |
| 任意字符 | 输入查询或 tip（搜索模式） |
| `Backspace` | 删除搜索输入（搜索模式） |
| `Esc` | 取消搜索 |
| `-` | 返回上一个 tab（普通模式） |
| `q` / `Esc` | 关闭 overview（普通模式） |
| `?` | 打开或关闭完整快捷键帮助 |
| `Ctrl+y` | 再次按下时关闭 overview |

## 已知限制

Zellij 会整体显示或隐藏 floating layer。打开 overview 时，原本隐藏的其他 floating
pane 也会暂时出现；关闭 overview 后，该图层会重新隐藏。目前 Zellij 插件 API
无法只显示一个可交互的 floating pane。

## 开发

```bash
cargo fmt --check
cargo lint
cargo test --lib
cargo wasm
zellij -l zellij.kdl
```

开发布局会从 `target/wasm32-wasip1/release/overview.wasm` 启动插件。界面通过
Ratatui Buffer 渲染，Zellij 适配层负责读取 tab 状态和执行跳转。

## 发布

安装 [`cargo-release`](https://github.com/crate-ci/cargo-release)，先预演再发布：

```bash
cargo install cargo-release
cargo release patch
cargo release patch --execute
```

推送 `v*` tag 后，GitHub Actions 会验证项目、构建 WASM、生成 SHA-256 并创建
GitHub Release。

## 许可证

本项目采用 [GNU Affero General Public License v3.0 only](LICENSE)。
