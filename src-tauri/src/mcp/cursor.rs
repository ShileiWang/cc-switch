//! Cursor IDE 的 MCP 同步：读写 `~/.cursor/mcp.json`。
//!
//! Cursor 使用和 Claude Desktop 一样的 `mcpServers` 顶层键格式：
//! ```json
//! {
//!   "mcpServers": {
//!     "server-id": { ... }
//!   }
//! }
//! ```

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde_json::{Map, Value};

use crate::config::get_cursor_mcp_path;
use crate::database::Database;
use crate::error::AppError;

use super::validation::validate_mcp_server_config;

/// 读 `~/.cursor/mcp.json`，空文件或不存在返回空 servers 映射。
fn read_servers(path: &std::path::Path) -> Result<BTreeMap<String, Value>, AppError> {
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let content = std::fs::read_to_string(path).map_err(|e| AppError::Io(e.to_string()))?;
    if content.trim().is_empty() {
        return Ok(BTreeMap::new());
    }
    let doc: Value = serde_json::from_str(&content).map_err(|e| {
        AppError::localized(
            "cursor.mcp.parse_error",
            format!("Cursor MCP 配置解析失败: {e}"),
            format!("Failed to parse Cursor MCP config: {e}"),
        )
    })?;
    let servers = doc
        .pointer("/mcpServers")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    Ok(servers
        .into_iter()
        .map(|(k, v)| (k, v))
        .collect::<BTreeMap<_, _>>())
}

/// 写 servers 到 `~/.cursor/mcp.json`。
fn write_servers(path: &std::path::Path, servers: &BTreeMap<String, Value>) -> Result<(), AppError> {
    let parent = path.parent();
    if let Some(dir) = parent {
        if !dir.exists() {
            std::fs::create_dir_all(dir).map_err(|e| AppError::Io(e.to_string()))?;
        }
    }
    let doc = if servers.is_empty() {
        json!({})
    } else {
        let mut map = Map::new();
        let mut mcp_servers = Map::new();
        for (k, v) in servers {
            mcp_servers.insert(k.clone(), v.clone());
        }
        map.insert("mcpServers".to_string(), Value::Object(mcp_servers));
        Value::Object(map)
    };
    let content = serde_json::to_string_pretty(&doc).map_err(|e| AppError::Io(e.to_string()))?;
    std::fs::write(path, content).map_err(|e| AppError::Io(e.to_string()))?;
    Ok(())
}

/// 路径类型别名（方便测试）
pub type CursorPath = PathBuf;

/// 同步单个服务器到 Cursor MCP 配置。
pub fn sync(
    _path: &CursorPath,
    server_id: &str,
    server_spec: Option<&Value>,
) -> Result<(), AppError> {
    let path = get_cursor_mcp_path();
    let mut servers = read_servers(&path)?;
    match server_spec {
        Some(spec) => {
            validate_mcp_server_config(spec)?;
            servers.insert(server_id.to_string(), spec.clone());
        }
        None => {
            servers.remove(server_id);
        }
    }
    write_servers(&path, &servers)?;
    Ok(())
}

/// 把数据库里 Cursor 启用的所有 MCP 服务器写入文件。
pub fn sync_all(db: &Database) -> Result<(), AppError> {
    let path = get_cursor_mcp_path();
    let all = db.get_all_mcp_servers()?;
    let mut servers = BTreeMap::new();
    for (id, server) in &all {
        if server.apps.cursor {
            servers.insert(id.clone(), server.server.clone());
        }
    }
    write_servers(&path, &servers)?;
    Ok(())
}
