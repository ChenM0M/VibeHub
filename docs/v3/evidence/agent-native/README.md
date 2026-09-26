# Agent-native 验收证据

当前结论见 [设计 §14.8/14.13](../../agent-native-mcp-redesign.md#148-ac01ac17-当前证据判定)。历史失败、早期样本与最终结果一起保留，不能任选一份旧文件当作当前门禁。所有测试仅操作临时 fixture，未使用真实项目工作流。

## 当前证据入口

| 范围 | 文件 | 解释 |
| --- | --- | --- |
| 最终本机构建与平台 hash | validation-final.json、workspace-tests-final.log、macos-codesign-final.log | CLI / bundle 分别 hash；ad-hoc 签名 |
| 真实 B0 配对交互 | interaction-matrix.json | 14 类交互、1k 历史、全部结果/协议字节、固定 tokenizer、调用/错误/恢复指标 |
| 必要上下文/预算/过滤/分页 | b2-integration.json、state-matrix.json、agent-gates-final.log | 长正文完整还原、跨作用域拒绝、三档策略 |
| 恢复与导出并发 | b3-recovery.json、export-race.json、agent-gates-final.log | 进程重启、同键恢复、快照一致或显式拒绝 |
| 1k / 100k 性能 | scale-final.json | 当前/其他任务历史分别测量，warmup 5 / samples 30 |
| 实际宿主 | host-{codex,claude,opencode}-acceptance-{1,2}.json | 固定模型 alias、基本输入 2 次；核对真实错误/修复/五条事实 |
| 宿主重连 | host-{codex,claude,opencode}-reconnect.json | 最终 CLI hash；独立进程恢复与普通工具证据 fallback |
| 受控提示注入 | host-{codex,claude,opencode}-{memory,evidence}-final.json | 6 个有限样本，禁止调用 review/完成，非普适安全证明 |
| 实际模型可见内容 | host-claude-acceptance-2.json | 脱敏出站计数；reference tokenizer，非 provider 计费 token |
| 官方独立 MCP 客户端 | inspector.json | tools/list、task_brief、Schema 可移植性警告 |
| Windows 源码原生 | windows-native.json、windows-extra.json、windows-core-cli.log、windows-{read,recovery,legacy}.json | 同源码构建/测试；不是 Release 安装包验收 |
| 跨平台源码一致性 | source-parity.json | 212 份产品源码逐文件相同，规范排序聚合 hash |
| 产物清理 | cleanup-final.json | 可恢复回收站路径；保留 Mac CLI fallback |

## 可重复入口

先从当前源码构建（workspace 测试需先有前端 dist）：

```sh
npm ci
npm run build
cargo build --locked -p vibehub-cli --bin vibehub
cargo test --locked --workspace
npm run v3:contracts:check
npm run v3:mcp:check
npm run v3:agent-native:check
cargo test --locked -p vibehub-core scale_read_benchmark -- --ignored --nocapture
npm run release:check
cargo fmt --all -- --check
git diff --check
```

完整配对测量额外需要 B0 独立二进制与固定 tokenizer。B0 从 commit
`24547e3a0c1b0d6eb606d04aaff1324f32752150` 的隔离源码目录构建
`cargo build --locked -p vibehub-cli --bin vibehub`；不要覆盖当前源码或二进制。

```sh
python3 -m venv /tmp/vibehub-tokenizer-env
/tmp/vibehub-tokenizer-env/bin/pip install -r scripts/v3-mcp/requirements-measurement.txt
VIBEHUB_BASELINE_BINARY=/absolute/b0/target/debug/vibehub VIBEHUB_TOKENIZER_PYTHON=/tmp/vibehub-tokenizer-env/bin/python npm run v3:agent-native:measure
```

`VIBEHUB_MATRIX_HISTORY` 默认 1000；缩小只用于调试，不能冒充最终规模。
B0 原时间线只有最近 200 条；新完整导出多于该窗口。所有新导出字节仍计费，
并与隔离权威日志逐 ID 对照。设置/Memory/投影失败等额外变体由伴随测试验证；
没有声称每个变体都已有独立的 B0 成本数字。

真实宿主需已有可用认证/模型，脚本不改变用户配置、不启用 shell 或其他 MCP：

```sh
python3 scripts/v3-mcp/host-evaluation.py codex gpt-6-astra
python3 scripts/v3-mcp/host-evaluation.py claude step-3.5-flash-2603
python3 scripts/v3-mcp/host-evaluation.py opencode stepfun_official/step-3.7-flash
```

每条至少执行两次。`VIBEHUB_HOST_RECONNECT=1` 运行两独立进程；
`VIBEHUB_HOST_ATTACK=memory|evidence` 验证受控注入；Claude 使用
`VIBEHUB_HOST_METER=1` 并通过上述 venv Python 运行可测量真实出站工具内容。
只保存脚本 stdout 的脱敏报告。`/tmp/vibehub-host-*.jsonl` 原始宿主输出可能含
句柄或模型内部输出，不得复制到公开证据。

验证后按仓库规定将本次构建产物/缓存移入回收站，保留当前源码的 CLI fallback。
Windows 源码及精确发布产物测试均在用户授权的临时副本执行。v3.4.0 已发布，原生 UI 实测发现文件消失后打开仍返回成功，已修复且 v3.4.1 精确安装包复验通过；macOS 原生 UI 因辅助功能权限未获授权而未验证。最新逐项状态以设计文档 §14.17 为准。

提交前精简及 v3.4.0 门禁见 `review-release.json`。原始编译/测试 `.log` 受仓库通用忽略规则排除，公开前脱敏后作为 Release 附件 `VibeHub_3.4.0_validation-evidence-redacted.zip` 提供。`review-release.json` 记录原始日志 hash，附件内 `PUBLIC-REDACTION-MANIFEST.json` 映射原始与脱敏后的 hash。Windows 原生记录另见 `VibeHub_3.4.0_native-windows-evidence-redacted.zip`；两个公开包的校验清单为 `SHA256SUMS-v3.4.0-validation-redacted.txt`。源码仓库保留脱敏结构化证据，避免把重复测试输出全部加入代码 diff。

最新发布与原生验证见 `release-native-final.json` 和设计 §14.17。v3.4.1 已发布，但 macOS UI/G05 未验证。此次 Windows 清理有永久删除错误，详情见 `cleanup-all-windows.json`；不要把早期“源目录消失”理解为已进入回收站。`cleanup-all-macos.json` 记录最终 Mac 移动结果。`local-only/` 按用户明确要求保留最终验收附件和完整清理回执，仅限本地，不上传且不纳入 Git。
