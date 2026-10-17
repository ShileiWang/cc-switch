//! 写 Cursor IDE 的 `settings.json`：只替换 `cursor.general.*` 关键字段，
//! 其余用户设置不碰。
//!
//! 写 Cursor live 的入口（切换、新增第一个供应商、编辑当前供应商、同步、统一供应商、
//! 进入 / 退出代理）都走这里：先拿应用写锁，再经 `mode::operation` 记下 pending，
//! 文件和状态在同一个操作里提交。

use crate::app_config::AppType;
use crate::config::get_cursor_settings_path;
use crate::database::Database;
use crate::error::AppError;
use crate::live::engine::LiveFile;
use crate::live::patch::LivePatch;
use crate::live::project::cursor::CursorProjection;
use crate::mode::operation::{AppWrite, FileChange, OperationReport};
use crate::mode::state::{op, PendingTarget};
use crate::provider::Provider;

fn app() -> &'static str {
    AppType::Cursor.as_str()
}

/// Cursor `settings.json`：里面可能有 API Key，按 private 写。
pub(crate) fn settings_file() -> LiveFile {
    LiveFile::private(get_cursor_settings_path())
}

/// 官方卡：按分类判断
pub(crate) fn is_official(provider: &Provider) -> bool {
    provider.category.as_deref() == Some("official")
}

/// 供应商的关键字段投影。
pub(crate) fn projection(provider: &Provider) -> Result<CursorProjection, AppError> {
    Ok(CursorProjection::of(&provider.settings_config))
}

/// 切到 `target`：同一个操作里写 live、再把当前供应商改成它。
pub(crate) fn switch_to(db: &Database, target: &Provider) -> Result<OperationReport, AppError> {
    let proj = projection(target)?;
    run(
        db,
        op::SWITCH,
        Some(&proj),
        PendingTarget::pointer(Some(target.id.clone())),
    )
}

/// 把当前供应商重新投影到 live，不改指针。
pub(crate) fn reapply(db: &Database, target: &Provider) -> Result<OperationReport, AppError> {
    let proj = projection(target)?;
    run(db, op::APPLY, Some(&proj), PendingTarget::default())
}

/// 把 `projection` 写进 settings.json，和 `target` 在同一个操作里提交；
/// `projection` 为空时只落定状态、不读也不写文件。
pub(crate) fn run(
    db: &Database,
    op_name: &str,
    projection: Option<&CursorProjection>,
    target: PendingTarget,
) -> Result<OperationReport, AppError> {
    let write = AppWrite::begin(db, app())?;
    let patch = projection.map(|p| LivePatch {
        bytes: get_cursor_settings_path().clone(),
        patch: p.settings_patch().into(),
    });
    let file = settings_file();
    let changes = [FileChange::new(&file, patch.as_ref())];
    write.apply(op_name, &changes, &target)
}
