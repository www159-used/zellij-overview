# overview 产品需求

用户手册（已上线行为）见 [README](README.md) 和 [英文 README](../../README.md)。本文是目标和需求，含尚未做的部分。

## 目标

在 Zellij 里**尽快跳到指定 session / tab**。看板是认位置用的，跳转是主路径。

典型用法：`ww` 是日常 session；`lp` 是项目 session，worktree 是 tab。人经常在 `ww`，要一次落到 `lp` 里某个 wt。

## 已上线（以 README 为准）

- 一块板：session 卡钉在最前（`◆` + tab 数），后面是**当前** session 的 tab。
- 够宽用框，中间丢掉框，不够宽一列滚动（session `◆`，tab 缩进）。
- `s` / `e` 两级：落在 session 上只换成该 session 的 tab 板，不离开 overview；再在 tab 上才跳。当前 session 的 tab 在首页就能直接跳。
- 别人的 tab 用 `switch_session_with_focus`；当前 session 用 `go_to_tab`。
- 从别人的 tab 板 `Esc` / `q`：先回首页，再按才关。钻进的板不再放其它 session 卡。
- `-` / `[-]`：同 session 用 Zellij tab 历史。首页 `[-]` 在别人的 session 上时，`-` 和 `e` 一样先进入该 session 的 tab 板；板上的 `[-]` 是该 session 上次的 tab，再 `-` 才跳（`/cache/previous`、`/cache/session-last`）。同 session 里再跳过 tab 后，清掉这份离开的 session，`-` 重新落在上一个 tab。
- 没有 pane 层，没有 `Space` leader。
- 打开时拉一次 `get_session_list()`；进入缺 tab 的 session 板时再拉；`SessionUpdate` 只有当前 session 时不冲掉别人。
- 关掉时往 `/cache/usage.jsonl` 追加一条（键次数、Flash / hjkl / `-` / 是否钻进 session 板、结局、是否跨 session）。不记标题，不上传，最多 400 条。macOS 在 `~/Library/Caches/org.Zellij-Contributors.Zellij`，Linux 在 `~/.cache/zellij`。本机 `./scripts/usage-summary.sh` 汇总后再看要不要砍键、要不要打开即搜。

## 已对齐、未做

（无。打开即 Flash、砍 `hjkl`、Space 认位置先靠用量再定。）

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
- `Ctrl+y` `-` 先进入离开的 session 的 tab 板，再 `-` 落到上次的 tab。
- 同 session 里 Flash / `e` 不比现在慢。
