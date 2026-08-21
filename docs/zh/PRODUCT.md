# overview 产品需求

用户手册（已上线行为）见 [README](README.md) 和 [英文 README](../../README.md)。本文是目标和需求，含尚未做的部分。

## 目标

在 Zellij 里**尽快跳到指定 session / tab**。看板是认位置用的，跳转是主路径。

典型用法：`ww` 是日常 session；`lp` 是项目 session，worktree 是 tab。人经常在 `ww`，要一次落到 `lp` 里某个 wt。

## 已上线（以 README 为准）

- 一块板：session 卡钉在最前（`◆` + tab 数），后面是**当前** session 的 tab。
- 够宽用框，中间丢掉框，不够宽一列滚动（session `◆`，tab 缩进）。
- `s` Flash：搜当前板上的标题，tip 直接跳。
- `e` / `Enter`：session 卡会 `switch_session`（不带 tab）；当前 tab 会 `go_to_tab`。
- `-` / `[-]`：同 session 用 Zellij tab 历史；切走过 session 后标离开的 session（`/cache/previous`）。
- 没有 pane 层，没有 `Space` leader。
- 打开时拉一次 `get_session_list()`；`SessionUpdate` 只有当前 session 时不冲掉别人。

## 已对齐、未做

### 1. 跨 session 少一跳（当前主痛点）

`s` / `e` 都是两级，**落在 session 上不离开 overview**：

1. 在 session 上 `s` 的 tip 或 `e` → 换成该 session 的 tab 板，不 `switch_session`。
2. 再在 tab 上 `s` / `e` → 当前 session 用 `go_to_tab`，别人的用 `switch_session_with_focus`。

当前 session 的 tab 仍可直接跳，不必先再选一遍自己。

从别人的 tab 板 `Esc`：先回到原来的板，再按才关。

不把所有 session 的 tab 摊在第一块板上。

### 2. 每个 session 上次的 tab

`/cache` 按 session 记上次 tab。该 session 的 tab 板上标 `[-]`。`-` 和点那张卡的 `e` 一样：一次落到那个 session 的那个 tab。

### 3. 本机用量（决定砍不砍键）

先不砍 `hjkl`。关掉时往 `/cache` 追加一条（按键次数、是否 Flash、是否 hjkl、结局、是否跨 session、是否 `-`）。不记标题，不上传，日志封顶。本机汇总后再看要不要砍键、要不要打开即搜。

## 明确不做（除非以后改口）

- 打开默认进 Flash。
- 默认展开所有 session 的整棵树，或扁平列出全部 tab。
- pane 层 / `Space` leader。
- 未看用量就砍 `hjkl`。
- 深历史栈（多步 `-`）。先做好「上一项」。

## 约束

- Zellij 0.44+ 插件；跨 session 状态用 `/cache`（不是 `/data`）。
- `get_session_list()` 不能跟在每次 `update` 后面（会抢 stdin）。打开时或进入某个 session 的 tab 板时拉。
- 离开 session 时 Zellij 会清该 client 的 `tab_history`，跨 session 的 `[-]` 不能只靠它。
- 浮动层整层显隐；关掉要按打开前的状态恢复。

## 怎样算达到目标

- 人在 `ww`，要去 `lp` 的某个 tab：打开 **一次** overview 就能落到。
- `Ctrl+y` `-` 能回到上一个工作点（tab，不只是 session 卡）。
- 同 session 里 Flash / `e` 不比现在慢。
