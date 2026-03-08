//! Parse and update labeled LLM preset blocks in `.env`.
//!
//! Presets are defined as comment-labeled blocks such as:
//!
//! ```text
//! # aliyun
//! # LLM_BACKEND=openai_compatible
//! # LLM_BASE_URL=https://...
//! # LLM_API_KEY=...
//! # LLM_MODEL=qwen3.5-plus
//! ```
//!
//! Activating a preset comments out the provider keys in all preset blocks
//! and uncomments the target block.

use std::path::{Path, PathBuf};

use crate::bootstrap::ironclaw_env_path;

const PROVIDER_KEYS: &[&str] = &[
    "LLM_BACKEND",
    "LLM_BASE_URL",
    "LLM_API_KEY",
    "LLM_MODEL",
    "ANTHROPIC_BASE_URL",
    "ANTHROPIC_API_KEY",
    "ANTHROPIC_MODEL",
];

#[derive(Debug, thiserror::Error)]
pub enum LlmPresetError {
    #[error("No .env file found in the current project or ~/.ironclaw")]
    EnvFileNotFound,
    #[error("No LLM presets were found in {0}")]
    NoPresetsFound(String),
    #[error("Unknown LLM preset '{0}'")]
    UnknownPreset(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmPresetSummary {
    pub label: String,
    pub backend: String,
    pub model: String,
    pub base_url: Option<String>,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmPresetList {
    pub source_path: PathBuf,
    pub active_label: Option<String>,
    pub presets: Vec<LlmPresetSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmPresetSelection {
    pub source_path: PathBuf,
    pub active_label: String,
    pub changed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PresetLine {
    line_index: usize,
    key: String,
    value: String,
    commented: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PresetBlock {
    label: String,
    lines: Vec<PresetLine>,
}

impl PresetBlock {
    fn active(&self) -> bool {
        self.lines.iter().any(|line| !line.commented)
    }

    fn backend(&self) -> String {
        self.find_value("LLM_BACKEND")
            .or_else(|| {
                if self.find_value("ANTHROPIC_BASE_URL").is_some()
                    || self.find_value("ANTHROPIC_MODEL").is_some()
                {
                    Some("anthropic".to_string())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "openai_compatible".to_string())
    }

    fn model(&self) -> String {
        self.find_value("LLM_MODEL")
            .or_else(|| self.find_value("ANTHROPIC_MODEL"))
            .unwrap_or_else(|| "unconfigured".to_string())
    }

    fn base_url(&self) -> Option<String> {
        self.find_value("LLM_BASE_URL")
            .or_else(|| self.find_value("ANTHROPIC_BASE_URL"))
    }

    fn find_value(&self, key: &str) -> Option<String> {
        self.lines
            .iter()
            .find(|line| line.key == key)
            .map(|line| line.value.clone())
    }

    fn summary(&self) -> LlmPresetSummary {
        LlmPresetSummary {
            label: self.label.clone(),
            backend: self.backend(),
            model: self.model(),
            base_url: self.base_url(),
            active: self.active(),
        }
    }
}

pub fn list_llm_presets() -> Result<LlmPresetList, LlmPresetError> {
    let path = resolve_env_path()?;
    list_llm_presets_at(&path)
}

pub fn select_llm_preset(label: &str) -> Result<LlmPresetSelection, LlmPresetError> {
    let path = resolve_env_path()?;
    select_llm_preset_at(&path, label)
}

fn resolve_env_path() -> Result<PathBuf, LlmPresetError> {
    let project_env = std::env::current_dir()?.join(".env");
    if project_env.exists() {
        return Ok(project_env);
    }

    let bootstrap_env = ironclaw_env_path();
    if bootstrap_env.exists() {
        return Ok(bootstrap_env);
    }

    Err(LlmPresetError::EnvFileNotFound)
}

fn list_llm_presets_at(path: &Path) -> Result<LlmPresetList, LlmPresetError> {
    let content = std::fs::read_to_string(path)?;
    let presets = parse_presets(&content);
    if presets.is_empty() {
        return Err(LlmPresetError::NoPresetsFound(path.display().to_string()));
    }

    let summaries: Vec<LlmPresetSummary> = presets.iter().map(PresetBlock::summary).collect();
    let active_label = summaries.iter().find(|preset| preset.active).map(|p| p.label.clone());

    Ok(LlmPresetList {
        source_path: path.to_path_buf(),
        active_label,
        presets: summaries,
    })
}

fn select_llm_preset_at(path: &Path, label: &str) -> Result<LlmPresetSelection, LlmPresetError> {
    let content = std::fs::read_to_string(path)?;
    let presets = parse_presets(&content);
    if presets.is_empty() {
        return Err(LlmPresetError::NoPresetsFound(path.display().to_string()));
    }

    let target_exists = presets.iter().any(|preset| preset.label == label);
    if !target_exists {
        return Err(LlmPresetError::UnknownPreset(label.to_string()));
    }

    let previously_active = presets.iter().find(|preset| preset.active()).map(|p| p.label.clone());
    let mut lines: Vec<String> = content.split('\n').map(|line| line.to_string()).collect();

    for preset in &presets {
        let should_activate = preset.label == label;
        for preset_line in &preset.lines {
            lines[preset_line.line_index] = format_line(&preset_line.key, &preset_line.value, !should_activate);
        }
    }

    let mut updated = lines.join("\n");
    if content.ends_with('\n') && !updated.ends_with('\n') {
        updated.push('\n');
    }

    if updated != content {
        std::fs::write(path, updated)?;
    }

    Ok(LlmPresetSelection {
        source_path: path.to_path_buf(),
        active_label: label.to_string(),
        changed: previously_active.as_deref() != Some(label),
    })
}

fn parse_presets(content: &str) -> Vec<PresetBlock> {
    let mut presets = Vec::new();
    let mut pending_label: Option<String> = None;
    let mut current: Option<PresetBlock> = None;

    for (line_index, raw_line) in content.split('\n').enumerate() {
        let trimmed = raw_line.trim();

        if trimmed.is_empty() {
            flush_preset(&mut presets, &mut current);
            pending_label = None;
            continue;
        }

        if let Some(comment_body) = trimmed.strip_prefix('#') {
            let comment = comment_body.trim();
            if !comment.is_empty() && parse_provider_assignment(comment).is_none() {
                flush_preset(&mut presets, &mut current);
                pending_label = Some(comment.to_string());
                continue;
            }
        }

        if let Some((commented, key, value)) = parse_provider_line(trimmed) {
            let block = current.get_or_insert_with(|| PresetBlock {
                label: pending_label
                    .clone()
                    .unwrap_or_else(|| "active".to_string()),
                lines: Vec::new(),
            });
            block.lines.push(PresetLine {
                line_index,
                key,
                value,
                commented,
            });
            continue;
        }

        flush_preset(&mut presets, &mut current);
        pending_label = None;
    }

    flush_preset(&mut presets, &mut current);
    presets
}

fn flush_preset(presets: &mut Vec<PresetBlock>, current: &mut Option<PresetBlock>) {
    if let Some(block) = current.take()
        && !block.lines.is_empty()
    {
        presets.push(block);
    }
}

fn parse_provider_assignment(line: &str) -> Option<(String, String)> {
    let (key, value) = line.split_once('=')?;
    let key = key.trim();
    if !PROVIDER_KEYS.contains(&key) {
        return None;
    }
    Some((key.to_string(), value.trim().to_string()))
}

fn parse_provider_line(line: &str) -> Option<(bool, String, String)> {
    let commented = line.starts_with('#');
    let candidate = if commented {
        line.trim_start_matches('#').trim_start()
    } else {
        line
    };
    let (key, value) = parse_provider_assignment(candidate)?;
    Some((commented, key, value))
}

fn format_line(key: &str, value: &str, commented: bool) -> String {
    if commented {
        format!("# {key}={value}")
    } else {
        format!("{key}={value}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_env() -> String {
        r#"# zhipu
# LLM_BACKEND=anthropic
# ANTHROPIC_BASE_URL=https://open.bigmodel.cn/api/anthropic
# ANTHROPIC_API_KEY=secret
# ANTHROPIC_MODEL=glm-5

# minimaxi
LLM_BACKEND=openai_compatible
LLM_BASE_URL=https://api.minimaxi.com/v1
LLM_API_KEY=secret2
LLM_MODEL=MiniMax-M2.5

# aliyun
# LLM_BACKEND=openai_compatible
# LLM_BASE_URL=https://dashscope.aliyuncs.com/compatible-mode/v1
# LLM_API_KEY=secret3
# LLM_MODEL=qwen3.5-plus
"#
        .to_string()
    }

    #[test]
    fn parse_presets_detects_active_block() {
        let presets = parse_presets(&sample_env());
        assert_eq!(presets.len(), 3);
        assert_eq!(presets[1].label, "minimaxi");
        assert!(presets[1].active());
        assert_eq!(presets[2].summary().model, "qwen3.5-plus");
    }

    #[test]
    fn select_preset_rewrites_only_provider_blocks() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join(".env");
        std::fs::write(&path, sample_env()).expect("write env");

        let result = select_llm_preset_at(&path, "aliyun").expect("select aliyun");
        assert_eq!(result.active_label, "aliyun");
        assert!(result.changed);

        let updated = std::fs::read_to_string(&path).expect("read env");
        assert!(updated.contains("# LLM_MODEL=MiniMax-M2.5"));
        assert!(updated.contains("LLM_MODEL=qwen3.5-plus"));
        assert!(updated.contains("# ANTHROPIC_MODEL=glm-5"));
    }

    #[test]
    fn list_presets_reports_source_path() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join(".env");
        std::fs::write(&path, sample_env()).expect("write env");

        let listed = list_llm_presets_at(&path).expect("list presets");
        assert_eq!(listed.source_path, path);
        assert_eq!(listed.active_label.as_deref(), Some("minimaxi"));
    }
}
