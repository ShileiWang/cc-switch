//! Cursor IDE 的投影：供应商行 → `settings.json` 的 `cursor.general.*` 键。
//!
//! Cursor BYOK 模式的键都在 `cursor.general` 这个嵌套对象下：
//! - `cursor.general.openAIAPIKey` — OpenAI 兼容 API key
//! - `cursor.general.openAIBaseURL` — OpenAI 兼容 API base URL
//! - `cursor.general.anthropicApiKey` — Anthropic API key
//! - `cursor.general.googleApiKey` — Google API key
//!
//! 切走时按写入记录清空这些字段；代理契约写本地代理的 base URL 和占位 Key。

use serde_json::{json, Map, Value};

use crate::live::patch::json::JsonPatch;
use crate::live::patch::KeyPath;

/// Cursor settings 中 CC Switch 会写的关键字段。
///
/// 这些都在父对象 `cursor.general` 下，补丁器通过 `ClearScope` 整块清空再写目标值。
pub const CURSOR_BASE_URL_KEY: &str = "openAIBaseURL";
pub const CURSOR_API_KEY_KEY: &str = "openAIAPIKey";
pub const CURSOR_ANTHROPIC_KEY: &str = "anthropicApiKey";
pub const CURSOR_GOOGLE_KEY: &str = "googleApiKey";

/// 所有 Cursor 关键字段名（在 `cursor.general` 下）。
pub const CURSOR_FLOOR_KEYS: &[&str] = &[
    CURSOR_BASE_URL_KEY,
    CURSOR_API_KEY_KEY,
    CURSOR_ANTHROPIC_KEY,
    CURSOR_GOOGLE_KEY,
];

/// 代理契约写进 Cursor settings 的占位 API Key 值。
pub const PROXY_TOKEN_PLACEHOLDER: &str = "CC_SWITCH_PROXY_MANAGED";

/// `cursor.general` 的 KeyPath。
pub fn cursor_general_path() -> KeyPath {
    KeyPath::new(&["cursor", "general"])
}

/// 一个供应商在 Cursor settings 里拥有的关键字段。
#[derive(Debug, Clone, PartialEq)]
pub struct CursorProjection {
    /// `cursor.general.openAIBaseURL`
    pub base_url: Option<String>,
    /// `cursor.general.openAIAPIKey`
    pub api_key: Option<String>,
    /// `cursor.general.anthropicApiKey`
    pub anthropic_api_key: Option<String>,
    /// `cursor.general.googleApiKey`
    pub google_api_key: Option<String>,
}

impl Default for CursorProjection {
    fn default() -> Self {
        Self {
            base_url: None,
            api_key: None,
            anthropic_api_key: None,
            google_api_key: None,
        }
    }
}

impl CursorProjection {
    /// 从供应商行取出要写的 Cursor settings 关键字段。
    ///
    /// 行里可以包含 `cursor_base_url`、`cursor_api_key`、`cursor_anthropic_api_key`、
    /// `cursor_google_api_key` 这些顶层键，也可以从通用的 `base_url` + `api_key` 推导。
    pub fn of(settings: &Value) -> Self {
        // 优先读 cursor 专属键
        let base_url = settings
            .pointer("/config/cursor/baseUrl")
            .and_then(Value::as_str)
            .or_else(|| settings.get("base_url").and_then(Value::as_str))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let api_key = settings
            .pointer("/config/cursor/apiKey")
            .and_then(Value::as_str)
            .or_else(|| settings.get("api_key").and_then(Value::as_str))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let anthropic_api_key = settings
            .pointer("/config/cursor/anthropicApiKey")
            .and_then(Value::as_str)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let google_api_key = settings
            .pointer("/config/cursor/googleApiKey")
            .and_then(Value::as_str)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        Self {
            base_url,
            api_key,
            anthropic_api_key,
            google_api_key,
        }
    }

    /// 只清空关键字段，不写任何值。
    pub fn empty() -> Self {
        Self::default()
    }

    /// 代理契约：本地代理地址、占位 Key。
    pub fn proxy_contract(proxy_base_url: &str) -> Self {
        Self {
            base_url: Some(proxy_base_url.to_string()),
            api_key: Some(PROXY_TOKEN_PLACEHOLDER.to_string()),
            anthropic_api_key: None,
            google_api_key: None,
        }
    }

    /// 摘要用的规范形式。
    pub fn to_value(&self) -> Value {
        let mut map = Map::new();
        if let Some(v) = &self.base_url {
            map.insert(CURSOR_BASE_URL_KEY.to_string(), json!(v));
        }
        if let Some(v) = &self.api_key {
            map.insert(CURSOR_API_KEY_KEY.to_string(), json!(v));
        }
        if let Some(v) = &self.anthropic_api_key {
            map.insert(CURSOR_ANTHROPIC_KEY.to_string(), json!(v));
        }
        if let Some(v) = &self.google_api_key {
            map.insert(CURSOR_GOOGLE_KEY.to_string(), json!(v));
        }
        Value::Object(map)
    }

    /// 生成 JSON 补丁：清空 `cursor.general` 下的所有 Cursor 关键字段，
    /// 然后写入当前 projection 里有的键。
    pub fn settings_patch(&self) -> JsonPatch {
        let general = cursor_general_path();
        let mut patch = JsonPatch::default();

        // 先清空 `cursor.general` 下所有已知的 Cursor 关键字段
        for key in CURSOR_FLOOR_KEYS {
            let full = general.child(key);
            match self.field(key) {
                Some(value) => {
                    // 目标也有这个键：原位改值（通过 set，不会触发 clear 的删除）
                    patch.set.push((full, Value::String(value.clone())));
                }
                None => {
                    // 目标没有：删掉它
                    patch.remove.push(full);
                }
            }
        }

        patch
    }

    fn field(&self, key: &str) -> Option<&str> {
        match key {
            CURSOR_BASE_URL_KEY => self.base_url.as_deref(),
            CURSOR_API_KEY_KEY => self.api_key.as_deref(),
            CURSOR_ANTHROPIC_KEY => self.anthropic_api_key.as_deref(),
            CURSOR_GOOGLE_KEY => self.google_api_key.as_deref(),
            _ => None,
        }
    }
}

/// 检查某个键是否是 Cursor settings 里的关键字段（用于 ClearScope）。
pub fn is_cursor_floor(key: &str) -> bool {
    CURSOR_FLOOR_KEYS.contains(&key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::live::patch::LivePatch;
    use std::path::Path;

    fn apply_patch(projection: &CursorProjection, pre: &str) -> Value {
        let patch = projection.settings_patch();
        let out = patch
            .apply(Path::new("settings.json"), Some(pre.as_bytes()))
            .unwrap();
        serde_json::from_slice(&out).unwrap()
    }

    #[test]
    fn projection_reads_base_fields_from_row() {
        let row = json!({
            "base_url": "https://a.example",
            "api_key": "sk-a",
        });
        let p = CursorProjection::of(&row);
        assert_eq!(p.base_url.as_deref(), Some("https://a.example"));
        assert_eq!(p.api_key.as_deref(), Some("sk-a"));
        assert!(p.anthropic_api_key.is_none());
        assert!(p.google_api_key.is_none());
    }

    #[test]
    fn projection_reads_cursor_specific_fields() {
        let row = json!({
            "config": {
                "cursor": {
                    "baseUrl": "https://c.example",
                    "apiKey": "sk-c",
                    "anthropicApiKey": "sk-ant",
                    "googleApiKey": "sk-google",
                }
            }
        });
        let p = CursorProjection::of(&row);
        assert_eq!(p.base_url.as_deref(), Some("https://c.example"));
        assert_eq!(p.api_key.as_deref(), Some("sk-c"));
        assert_eq!(p.anthropic_api_key.as_deref(), Some("sk-ant"));
        assert_eq!(p.google_api_key.as_deref(), Some("sk-google"));
    }

    #[test]
    fn proxy_contract_sets_local_url_and_placeholder() {
        let p = CursorProjection::proxy_contract("http://127.0.0.1:15721");
        assert_eq!(p.base_url.as_deref(), Some("http://127.0.0.1:15721"));
        assert_eq!(p.api_key.as_deref(), Some(PROXY_TOKEN_PLACEHOLDER));
        assert!(p.anthropic_api_key.is_none());
        assert!(p.google_api_key.is_none());
    }

    #[test]
    fn patch_writes_and_clears_cursor_general() {
        let pre = r#"{
            "cursor": {
                "general": {
                    "openAIBaseURL": "https://old.example",
                    "openAIAPIKey": "old-key",
                    "theme": "dark"
                }
            },
            "editor": {
                "fontSize": 14
            }
        }"#;
        let target = CursorProjection {
            base_url: Some("https://new.example".into()),
            api_key: Some("new-key".into()),
            ..Default::default()
        };
        let out = apply_patch(&target, pre);
        let general = &out["cursor"]["general"];
        assert_eq!(general["openAIBaseURL"].as_str(), Some("https://new.example"));
        assert_eq!(general["openAIAPIKey"].as_str(), Some("new-key"));
        // 用户自定义键不动
        assert_eq!(general["theme"].as_str(), Some("dark"));
        // Cursor 其他键被清掉
        assert!(general.get("anthropicApiKey").is_none());
        assert!(general.get("googleApiKey").is_none());
        // 其他顶层对象不动
        assert_eq!(out["editor"]["fontSize"].as_u64(), Some(14));
    }

    #[test]
    fn empty_projection_clears_all_floor_keys() {
        let pre = r#"{
            "cursor": {
                "general": {
                    "openAIBaseURL": "https://a.example",
                    "openAIAPIKey": "sk-a",
                    "anthropicApiKey": "sk-ant"
                }
            }
        }"#;
        let out = apply_patch(&CursorProjection::empty(), pre);
        let general = &out["cursor"]["general"];
        assert!(general.get("openAIBaseURL").is_none());
        assert!(general.get("openAIAPIKey").is_none());
        assert!(general.get("anthropicApiKey").is_none());
    }
}
