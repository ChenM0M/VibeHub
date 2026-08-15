# Agent Profiles 配置中心设计冻结（N01）

## 目标与边界

VibeHub 的配置中心只负责发现、读取、编辑、验证、激活和启动三个 Agent 的
受管配置：OpenCode、Claude Code、Codex。用户面对的是“哪个 Agent、哪个运行
环境、哪个 Profile、使用哪个 Provider/模型和思考档位”；协议直连、转换器和
sidecar 是后台能力，不作为用户必须理解或选择的配置项。

明确不纳入本功能：Cursor；登录账号切换；配额、计费和账号池；多实例负载均衡；
自动唤醒；重型网关控制台；读取或覆盖 Agent 的 auth、OAuth、credentials 文件。
API Key 只以环境变量、Keychain、Windows Credential Manager 或等价 credential
reference 出现，普通配置、项目文件、日志和导出中永远不出现密文。

## 统一领域契约

contracts/v3/agent-profile.schema.json 是唯一 wire contract，生成的
src/v3/contracts/generated/agent-profile.ts 只作为消费者类型。Adapter 不拥有
状态迁移规则，后续 Tauri typed command、CLI fallback 和 MCP 必须复用同一契约。

每个读取结果都必须同时携带：

- Agent、runtime target、实际 source path、格式和 scope；
- revision、内容 hash、schema/version capability；
- 受管字段、未受管字段、未知字段和注释保留能力；
- 默认状态及默认投影策略；
- native/upstream protocol、direct/adapter 路径和能力限制；
- freshness、completeness、warnings、errors 和 evidence refs。

unknown、unsupported、版本漂移和协议能力不匹配都是显式状态，不能自动映射为
“可用”。

## Agent 语义

| Agent | Profile 来源 | 临时启动 | 默认 Profile 行为 | 受管字段 |
| --- | --- | --- | --- | --- |
| OpenCode | 其用户/项目配置 | 后续由 Agent Adapter 决定 | 直接修改原生默认字段 | Provider、Base URL、credential reference、模型、默认/小模型、variants/思考档位 |
| Claude Code | VibeHub Profile 对应 settings | --settings path | 只投影受管字段到 settings，保留 permissions/hooks/MCP/sandbox | Provider、模型、协议字段及思考档位 |
| Codex | ~/.codex/<name>.config.toml | --profile name | 只投影受管字段到基础 config.toml | Provider、模型、协议字段及 reasoning/思考档位 |

Claude 和 Codex 的 Profile CRUD 不重写它们的未受管配置。删除默认 Profile 必须
先选择替代项或明确取消默认；Codex 不生成已移除的 [profiles.*] 语法。

OpenCode 的全局候选路径是 macOS/Linux 用户目录下
~/.config/opencode/opencode.jsonc 或 opencode.json，Windows 用户目录下
AppData/Roaming/opencode/opencode.jsonc 或 opencode.json。项目级配置还可以位于
项目根的 opencode.json(c) 或 .opencode/opencode.json(c)，并遵守 OpenCode 自身
的层级合并顺序。当前官方 schema 使用 provider 字段，历史/兼容配置可能使用
providers；Adapter 必须读取实际存在的字段并在原位置写回。OpenCode 根级默认模型
使用 model 和 small_model，Agent variant 使用 agent 配置中的 variant，不能
把 root model 中的 variant 伪装成官方默认字段。

## Runtime target

Runtime target 是配置操作的安全边界，不是一个装饰字段：

1. macOS host、Windows host 和每个 WSL distribution 都是独立 target；
2. path 必须来自目标环境实际 home/环境观察，不能由 host 名称猜能力；
3. target 记录 native path 与展示 path，展示层可以规范化分隔符，但 identity 不变；
4. 目标不可观察、权限不足、路径越界、符号链接/reparse point 不安全或 capability
   manifest 过期时，Adapter 必须返回 unavailable/unknown 并 fail closed。

## 用户交互流

~~~text
从左侧导航打开独立的“Agent Profiles”单页面
  └─ 选择 Agent → 选择 runtime target → 读取 Profile 列表
       ├─ 空状态：显示“发现配置 / 新建 Profile”
       ├─ 选择 Profile：显示默认标记、来源、兼容状态和行式模型列表
       ├─ 图形化新增/编辑/删除 Provider、模型和 variants → 本地校验 → 显示未保存变更
       ├─ 图形化新建/复制/重命名/删除 Profile，删除默认项必须先选替代项
       ├─ 保存：revision 校验 → 备份 → 原子写入 → 重新读取确认
       ├─ 设为默认：显示受管字段投影目标和保留范围 → 确认 → 激活
       ├─ 仅此次启动：解析协议 → 自动准备 sidecar → 启动 Agent
       └─ 高级兜底：查看/编辑原始 JSON/JSONC/TOML，仍复用同一校验和冲突门禁
~~~

### 必须可辨的状态

- empty：没有配置，但目标可写，可以创建；
- ready：配置可读取且 schema/protocol 能力完整；
- partial：部分字段可编辑，不支持字段只读并解释原因；
- stale：外部文件已变化，保存按钮转为重新读取/解决冲突；
- conflict：revision 不匹配，禁止覆盖外部新内容；
- unsupported：版本或协议未知，只展示安全诊断；
- error：权限、路径、语法、备份或 sidecar 失败，给出可恢复动作。

### 默认视图与高级视图

默认视图只展示 Provider、Base URL、credential reference、模型、默认/小模型、
思考档位和操作按钮；新增/编辑使用图形化 Dialog，不要求用户手写 JSON。高级视图
展示 source path、revision、保留字段摘要、协议诊断和原始配置，但不直接显示
Secret。内部 direct/adapter/sidecar 只在可展开的诊断中呈现，不形成额外的用户
决策步骤。

## Adapter 最小接口

后续 Adapter 必须实现等价于下面的 typed boundary；返回值只能是契约中的结构化
结果，不能让前端直接拼路径或改文件：

~~~text
discover(runtime_target) -> AgentProfileDiscoverResult
read(runtime_target, profile_id) -> AgentProfileReadResult
validate(profile) -> AgentProfileSaveResult | structured error
save(profile, expected_revision) -> AgentProfileSaveResult
activate(profile_id, expected_revision) -> AgentProfileSaveResult
launch(profile_id, temporary | default) -> AgentProfileSaveResult
~~~

保存顺序固定为：读取 revision → 校验 schema/能力/路径 → 同目录备份 → 临时文件
写入并原子替换 → 保持权限 → 重新读取确认；任一步失败都返回可恢复错误，不能
留下半写文件。
