use serde_yaml::Value;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VibehubLocale {
    En,
    ZhCn,
    ZhTw,
}

impl VibehubLocale {
    pub fn detect(project_root: &Path) -> Self {
        Self::detect_with_override(project_root, None)
    }

    pub fn detect_with_override(project_root: &Path, locale: Option<&str>) -> Self {
        locale
            .and_then(Self::parse)
            .or_else(|| {
                read_project_locale(project_root)
                    .as_deref()
                    .and_then(Self::parse)
            })
            .or_else(|| read_env_locale().as_deref().and_then(Self::parse))
            .unwrap_or(Self::En)
    }

    pub fn parse(raw: &str) -> Option<Self> {
        let value = raw.trim().replace('_', "-").to_ascii_lowercase();
        if value.is_empty() {
            return None;
        }
        if value.starts_with("zh-tw")
            || value.starts_with("zh-hk")
            || value.starts_with("zh-mo")
            || value.contains("hant")
        {
            return Some(Self::ZhTw);
        }
        if value == "zh"
            || value.starts_with("zh-cn")
            || value.starts_with("zh-sg")
            || value.contains("hans")
        {
            return Some(Self::ZhCn);
        }
        if value.starts_with("en") {
            return Some(Self::En);
        }
        None
    }

    pub fn evidence_grade_label(self) -> &'static str {
        match self {
            Self::En => "Evidence grade",
            Self::ZhCn => "证据等级",
            Self::ZhTw => "證據等級",
        }
    }

    pub fn none_observed(self) -> &'static str {
        match self {
            Self::En => "None observed.",
            Self::ZhCn => "未观察到。",
            Self::ZhTw => "未觀察到。",
        }
    }

    pub fn none(self) -> &'static str {
        match self {
            Self::En => "None.",
            Self::ZhCn => "无。",
            Self::ZhTw => "無。",
        }
    }

    pub fn none_lowercase(self) -> &'static str {
        match self {
            Self::En => "none",
            Self::ZhCn => "无",
            Self::ZhTw => "無",
        }
    }

    pub fn unavailable_git(self) -> &'static str {
        match self {
            Self::En => "unavailable (Git not detected)",
            Self::ZhCn => "不可用（未检测到 Git）",
            Self::ZhTw => "不可用（未偵測到 Git）",
        }
    }

    pub fn not_available(self) -> &'static str {
        match self {
            Self::En => "not available",
            Self::ZhCn => "不可用",
            Self::ZhTw => "不可用",
        }
    }

    pub fn not_reported(self) -> &'static str {
        match self {
            Self::En => "not reported by agent output.",
            Self::ZhCn => "agent 输出未报告。",
            Self::ZhTw => "agent 輸出未報告。",
        }
    }

    pub fn reported_suffix(self) -> &'static str {
        match self {
            Self::En => "reported",
            Self::ZhCn => "已报告",
            Self::ZhTw => "已報告",
        }
    }

    pub fn handoff_reported(self) -> &'static str {
        match self {
            Self::En => "Handoff notes: reported (see \"Next Session Should\" section)",
            Self::ZhCn => "交接笔记：已报告（见\"下次会话应\"章节）",
            Self::ZhTw => "交接筆記：已報告（見\"下次會話應\"章節）",
        }
    }

    pub fn handoff_not_reported(self) -> &'static str {
        match self {
            Self::En => "Handoff notes: not reported by agent output.",
            Self::ZhCn => "交接笔记：agent 输出未报告。",
            Self::ZhTw => "交接筆記：agent 輸出未報告。",
        }
    }

    pub fn context_complete_manifest(self) -> &'static str {
        match self {
            Self::En => "Context completeness: manifest available, quality flags assessed",
            Self::ZhCn => "上下文完整性：清单可用，已评估质量标记",
            Self::ZhTw => "上下文完整性：清單可用，已評估品質標記",
        }
    }

    pub fn context_complete_unknown(self) -> &'static str {
        match self {
            Self::En => "Context completeness: unknown (no manifest found)",
            Self::ZhCn => "上下文完整性：未知（未找到清单）",
            Self::ZhTw => "上下文完整性：未知（未找到清單）",
        }
    }

    pub fn research_available(self) -> &'static str {
        match self {
            Self::En => "Research evidence: research pack available",
            Self::ZhCn => "研究证据：研究包可用",
            Self::ZhTw => "研究證據：研究包可用",
        }
    }

    pub fn research_not_available(self) -> &'static str {
        match self {
            Self::En => "Research evidence: research pack not available",
            Self::ZhCn => "研究证据：研究包不可用",
            Self::ZhTw => "研究證據：研究包不可用",
        }
    }

    pub fn risk_no_output(self) -> &'static str {
        match self {
            Self::En => "Risk: no agent output.md found; evidence is incomplete",
            Self::ZhCn => "风险：未找到 agent output.md；证据不完整",
            Self::ZhTw => "風險：未找到 agent output.md；證據不完整",
        }
    }

    pub fn risk_git_unavailable(self) -> &'static str {
        match self {
            Self::En => "Risk: Git unavailable; changed files and diff are limited",
            Self::ZhCn => "风险：Git 不可用；变更文件和差异受限",
            Self::ZhTw => "風險：Git 不可用；變更檔案和差異受限",
        }
    }

    pub fn manifest_parse_error(self, path: &str) -> String {
        match self {
            Self::En => format!(
                "Context manifest path {} exists but could not be parsed.",
                path
            ),
            Self::ZhCn => format!("上下文清单路径 {} 存在但无法解析。", path),
            Self::ZhTw => format!("上下文清單路徑 {} 存在但無法解析。", path),
        }
    }

    pub fn tokens_used(self) -> &'static str {
        match self {
            Self::En => "tokens used",
            Self::ZhCn => "tokens 已使用",
            Self::ZhTw => "tokens 已使用",
        }
    }

    pub fn limit_label(self) -> &'static str {
        match self {
            Self::En => "limit",
            Self::ZhCn => "上限",
            Self::ZhTw => "上限",
        }
    }

    pub fn max_file_label(self) -> &'static str {
        match self {
            Self::En => "max file",
            Self::ZhCn => "最大文件",
            Self::ZhTw => "最大檔案",
        }
    }

    pub fn required(self) -> &'static str {
        match self {
            Self::En => "required",
            Self::ZhCn => "必要",
            Self::ZhTw => "必要",
        }
    }

    pub fn optional(self) -> &'static str {
        match self {
            Self::En => "optional",
            Self::ZhCn => "可选",
            Self::ZhTw => "可選",
        }
    }

    pub fn no_files_included(self) -> &'static str {
        match self {
            Self::En => "No files included.",
            Self::ZhCn => "无包含文件。",
            Self::ZhTw => "無包含檔案。",
        }
    }

    pub fn available_at(self) -> &'static str {
        match self {
            Self::En => "available at",
            Self::ZhCn => "可用路径",
            Self::ZhTw => "可用路徑",
        }
    }

    pub fn yes_file_exists(self) -> &'static str {
        match self {
            Self::En => "yes (file exists on filesystem)",
            Self::ZhCn => "是（文件存在于文件系统）",
            Self::ZhTw => "是（檔案存在於檔案系統）",
        }
    }

    pub fn present(self) -> &'static str {
        match self {
            Self::En => "present",
            Self::ZhCn => "存在",
            Self::ZhTw => "存在",
        }
    }

    pub fn missing_label(self) -> &'static str {
        match self {
            Self::En => "missing",
            Self::ZhCn => "缺失",
            Self::ZhTw => "缺失",
        }
    }

    pub fn research_not_analyzed(self) -> &'static str {
        match self {
            Self::En => "Research evidence analyzed: not automatically; full Research analysis not implemented in P0.",
            Self::ZhCn => "研究证据分析：未自动执行；P0 未实现完整研究分析。",
            Self::ZhTw => "研究證據分析：未自動執行；P0 未實現完整研究分析。",
        }
    }

    pub fn no_file_missing(self) -> &'static str {
        match self {
            Self::En => "no (file missing from filesystem)",
            Self::ZhCn => "否（文件系统中缺失文件）",
            Self::ZhTw => "否（檔案系統中缺失檔案）",
        }
    }

    pub fn research_missing_review(self) -> &'static str {
        match self {
            Self::En => "Research evidence missing: review cannot confirm research was performed.",
            Self::ZhCn => "研究证据缺失：审查无法确认是否已执行研究。",
            Self::ZhTw => "研究證據缺失：審查無法確認是否已執行研究。",
        }
    }

    pub fn no_diff_summary(self) -> &'static str {
        match self {
            Self::En => "No Git diff summary available or no changed files observed.",
            Self::ZhCn => "无 Git diff 摘要可用或未观察到变更文件。",
            Self::ZhTw => "無 Git diff 摘要可用或未觀察到變更檔案。",
        }
    }

    pub fn no_changed_files(self) -> &'static str {
        match self {
            Self::En => "No changed files observed by Git.",
            Self::ZhCn => "Git 未观察到变更文件。",
            Self::ZhTw => "Git 未觀察到變更檔案。",
        }
    }

    pub fn not_reported_by_session(self) -> &'static str {
        match self {
            Self::En => "Not reported by latest session output.",
            Self::ZhCn => "最新会话输出未报告。",
            Self::ZhTw => "最新會話輸出未報告。",
        }
    }

    pub fn limitation_p0(self) -> &'static str {
        match self {
            Self::En => "P0/P1 observability is best-effort.",
            Self::ZhCn => "P0/P1 可观测性为尽力而为。",
            Self::ZhTw => "P0/P1 可觀測性為盡力而為。",
        }
    }

    pub fn limitation_runtime(self) -> &'static str {
        match self {
            Self::En => "Runtime adapter observation is not available.",
            Self::ZhCn => "运行时适配器观察不可用。",
            Self::ZhTw => "執行時適配器觀察不可用。",
        }
    }

    pub fn limitation_tests(self) -> &'static str {
        match self {
            Self::En => "Tests and rationale are taken from agent/session reporting when present.",
            Self::ZhCn => "测试和理由在 agent/会话报告存在时取自该报告。",
            Self::ZhTw => "測試和理由在 agent/會話報告存在時取自該報告。",
        }
    }

    pub fn limitation_task_mapping(self) -> &'static str {
        match self {
            Self::En => "Task mapping is inferred from current YAML pointers and run location.",
            Self::ZhCn => "任务映射从当前 YAML 指针和运行位置推断。",
            Self::ZhTw => "任務映射從目前 YAML 指標和執行位置推斷。",
        }
    }

    pub fn limitation_git(self) -> &'static str {
        match self {
            Self::En => "Git was not available, so changed files and diff summary are limited.",
            Self::ZhCn => "Git 不可用，因此变更文件和差异摘要受限。",
            Self::ZhTw => "Git 不可用，因此變更檔案和差異摘要受限。",
        }
    }

    pub fn filesystem_presence(self) -> &'static str {
        match self {
            Self::En => "filesystem presence",
            Self::ZhCn => "文件系统存在",
            Self::ZhTw => "檔案系統存在",
        }
    }

    pub fn missing_required_sections(self) -> &'static str {
        match self {
            Self::En => "Missing required output sections",
            Self::ZhCn => "缺少必要输出章节",
            Self::ZhTw => "缺少必要輸出章節",
        }
    }

    pub fn verdict_blocked_no_evidence(self) -> &'static str {
        match self {
            Self::En => {
                "No agent output.md found and Git is not available. No evidence sources to review."
            }
            Self::ZhCn => "未找到 agent output.md 且 Git 不可用。无证据源可供审查。",
            Self::ZhTw => "未找到 agent output.md 且 Git 不可用。無證據源可供審查。",
        }
    }

    pub fn verdict_needs_output(self) -> &'static str {
        match self {
            Self::En => "No agent output.md found. Agent must produce run-level output before review can proceed.",
            Self::ZhCn => "未找到 agent output.md。Agent 必须生成运行级输出后才能进行审查。",
            Self::ZhTw => "未找到 agent output.md。Agent 必須產生執行級輸出後才能進行審查。",
        }
    }

    pub fn verdict_no_manifest(self) -> &'static str {
        match self {
            Self::En => {
                "Context manifest is not available; evidence completeness cannot be verified."
            }
            Self::ZhCn => "上下文清单不可用；无法验证证据完整性。",
            Self::ZhTw => "上下文清單不可用；無法驗證證據完整性。",
        }
    }

    pub fn verdict_missing_required_files(self, count: usize) -> String {
        match self {
            Self::En => format!("{} required context file(s) missing per manifest.", count),
            Self::ZhCn => format!("清单中有 {} 个必要上下文文件缺失。", count),
            Self::ZhTw => format!("清單中有 {} 個必要上下文檔案缺失。", count),
        }
    }

    pub fn context_quality_flags(self) -> &'static str {
        match self {
            Self::En => "Context quality flags",
            Self::ZhCn => "上下文质量标记",
            Self::ZhTw => "上下文品質標記",
        }
    }

    pub fn verdict_git_unavailable(self) -> &'static str {
        match self {
            Self::En => "Git is not available; changed files and diff are limited to filesystem observation.",
            Self::ZhCn => "Git 不可用；变更文件和差异仅限于文件系统观察。",
            Self::ZhTw => "Git 不可用；變更檔案和差異僅限於檔案系統觀察。",
        }
    }

    pub fn verdict_ready(self) -> &'static str {
        match self {
            Self::En => "All required evidence sources are present. VibeHub does not auto-pass tasks in P0; human review is required.",
            Self::ZhCn => "所有必要证据源均已就绪。VibeHub 在 P0 不会自动通过任务；需要人工审查。",
            Self::ZhTw => "所有必要證據源均已就緒。VibeHub 在 P0 不會自動通過任務；需要人工審查。",
        }
    }
}

fn read_project_locale(project_root: &Path) -> Option<String> {
    let state = fs::read_to_string(project_root.join(".vibehub/state.yaml")).ok()?;
    let state = serde_yaml::from_str::<Value>(&state).ok()?;
    yaml_string(&state, &["preferences", "locale"])
        .or_else(|| yaml_string(&state, &["settings", "locale"]))
        .or_else(|| yaml_string(&state, &["locale"]))
}

fn read_env_locale() -> Option<String> {
    [
        "VIBEHUB_LOCALE",
        "LC_ALL",
        "LC_MESSAGES",
        "LANG",
        "LANGUAGE",
    ]
    .iter()
    .find_map(|key| {
        std::env::var(key)
            .ok()
            .filter(|value| !value.trim().is_empty())
    })
}

pub fn persist_project_locale(project_root: &Path, locale: &str) -> anyhow::Result<()> {
    let state_path = project_root.join(".vibehub/state.yaml");
    let content = fs::read_to_string(&state_path)
        .map_err(|e| anyhow::anyhow!("Failed to read state.yaml: {}", e))?;
    let mut state: Value = serde_yaml::from_str(&content)
        .map_err(|e| anyhow::anyhow!("Failed to parse state.yaml: {}", e))?;

    // Ensure preferences section exists
    if state.get("preferences").is_none() {
        state.as_mapping_mut().unwrap().insert(
            Value::String("preferences".to_string()),
            Value::Mapping(serde_yaml::Mapping::new()),
        );
    }

    // Set locale in preferences
    if let Some(preferences) = state.get_mut("preferences") {
        if let Some(mapping) = preferences.as_mapping_mut() {
            mapping.insert(
                Value::String("locale".to_string()),
                Value::String(locale.to_string()),
            );
        }
    }

    let updated_content = serde_yaml::to_string(&state)
        .map_err(|e| anyhow::anyhow!("Failed to serialize state.yaml: {}", e))?;
    fs::write(&state_path, updated_content)
        .map_err(|e| anyhow::anyhow!("Failed to write state.yaml: {}", e))?;

    Ok(())
}

fn yaml_string(value: &Value, path: &[&str]) -> Option<String> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_str().map(ToString::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn parses_supported_locales() {
        assert_eq!(VibehubLocale::parse("en-US"), Some(VibehubLocale::En));
        assert_eq!(VibehubLocale::parse("zh-CN"), Some(VibehubLocale::ZhCn));
        assert_eq!(VibehubLocale::parse("zh"), Some(VibehubLocale::ZhCn));
        assert_eq!(VibehubLocale::parse("zh-Hans"), Some(VibehubLocale::ZhCn));
        assert_eq!(VibehubLocale::parse("zh-TW"), Some(VibehubLocale::ZhTw));
        assert_eq!(VibehubLocale::parse("zh-Hant"), Some(VibehubLocale::ZhTw));
    }

    #[test]
    fn project_locale_overrides_environment() {
        let project = std::env::temp_dir().join(format!("vibehub-locale-test-{}", Uuid::new_v4()));
        fs::create_dir_all(project.join(".vibehub")).expect("create");
        fs::write(
            project.join(".vibehub/state.yaml"),
            "preferences:\n  locale: zh-TW\n",
        )
        .expect("state");

        assert_eq!(VibehubLocale::detect(&project), VibehubLocale::ZhTw);

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn persist_project_locale_creates_preferences_if_missing() {
        let project = std::env::temp_dir().join(format!("vibehub-locale-test-{}", Uuid::new_v4()));
        fs::create_dir_all(project.join(".vibehub")).expect("create");
        fs::write(project.join(".vibehub/state.yaml"), "schema_version: 1\n").expect("state");

        persist_project_locale(&project, "zh-CN").expect("persist");

        let content = fs::read_to_string(project.join(".vibehub/state.yaml")).expect("read");
        let state: Value = serde_yaml::from_str(&content).expect("parse");
        assert_eq!(
            state
                .get("preferences")
                .unwrap()
                .get("locale")
                .unwrap()
                .as_str()
                .unwrap(),
            "zh-CN"
        );

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn persist_project_locale_updates_existing_preferences() {
        let project = std::env::temp_dir().join(format!("vibehub-locale-test-{}", Uuid::new_v4()));
        fs::create_dir_all(project.join(".vibehub")).expect("create");
        fs::write(
            project.join(".vibehub/state.yaml"),
            "preferences:\n  locale: en\n",
        )
        .expect("state");

        persist_project_locale(&project, "zh-TW").expect("persist");

        let content = fs::read_to_string(project.join(".vibehub/state.yaml")).expect("read");
        let state: Value = serde_yaml::from_str(&content).expect("parse");
        assert_eq!(
            state
                .get("preferences")
                .unwrap()
                .get("locale")
                .unwrap()
                .as_str()
                .unwrap(),
            "zh-TW"
        );

        fs::remove_dir_all(project).expect("cleanup");
    }
}
