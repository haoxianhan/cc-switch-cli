use clap::Subcommand;
use indexmap::IndexMap;

use crate::app_config::AppType;
use crate::cli::claude_temp_launch::{
    ensure_temp_launch_supported, exec_prepared_claude, prepare_launch, PreparedClaudeLaunch,
};
use crate::error::AppError;
use crate::provider::Provider;
use crate::services::ProviderService;
use crate::store::AppState;

#[derive(Subcommand)]
pub enum StartCommand {
    /// Start Claude with a provider selector without switching the global current provider
    Claude {
        /// Provider selector: exact ID first, then exact Name
        selector: String,
    },
}

pub fn execute(cmd: StartCommand) -> Result<(), AppError> {
    match cmd {
        StartCommand::Claude { selector } => start_claude(&selector),
    }
}

fn get_state() -> Result<AppState, AppError> {
    AppState::try_new()
}

fn start_claude(selector: &str) -> Result<(), AppError> {
    start_claude_with(
        selector,
        || {
            let state = get_state()?;
            ProviderService::list(&state, AppType::Claude)
        },
        |provider| {
            ensure_temp_launch_supported()?;
            let prepared = prepare_launch(provider, &std::env::temp_dir())?;
            handoff_and_cleanup(&prepared)
        },
    )
}

fn start_claude_with<Load, Launch>(
    selector: &str,
    load_providers: Load,
    launch_provider: Launch,
) -> Result<(), AppError>
where
    Load: FnOnce() -> Result<IndexMap<String, Provider>, AppError>,
    Launch: FnOnce(&Provider) -> Result<(), AppError>,
{
    let providers = load_providers()?;
    let provider = resolve_provider_selector(&providers, selector)?;
    launch_provider(&provider)
}

fn handoff_and_cleanup(prepared: &PreparedClaudeLaunch) -> Result<(), AppError> {
    let handoff_result = exec_prepared_claude(prepared);
    let cleanup_result = prepared.cleanup_settings_file();

    match (handoff_result, cleanup_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Ok(()), Err(cleanup_err)) => Err(cleanup_err),
        (Err(err), Ok(())) => Err(err),
        (Err(err), Err(cleanup_err)) => Err(AppError::localized(
            "claude.temp_launch_cleanup_failed",
            format!("启动 Claude 失败: {err}；同时清理临时设置文件失败: {cleanup_err}"),
            format!(
                "Failed to launch Claude: {err}; also failed to remove the temporary settings file: {cleanup_err}"
            ),
        )),
    }
}

fn resolve_provider_selector(
    providers: &IndexMap<String, Provider>,
    selector: &str,
) -> Result<Provider, AppError> {
    if let Some(provider) = providers.get(selector) {
        return Ok(provider.clone());
    }

    let exact_name_matches: Vec<_> = providers
        .values()
        .filter(|provider| provider.name == selector)
        .cloned()
        .collect();

    match exact_name_matches.as_slice() {
        [] => Err(AppError::localized(
            "cli.start.provider_selector_not_found",
            format!(
                "供应商选择器 '{}' 未匹配到任何 Claude 供应商的 ID 或名称",
                selector
            ),
            format!(
                "Provider selector '{}' did not match any Claude provider by ID or name",
                selector
            ),
        )),
        [provider] => Ok(provider.clone()),
        matches => {
            let ids = matches
                .iter()
                .map(|provider| provider.id.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            Err(AppError::localized(
                "cli.start.provider_selector_ambiguous",
                format!(
                    "供应商选择器 '{}' 存在歧义，匹配到这些 ID: {}",
                    selector, ids
                ),
                format!(
                    "Provider selector '{}' is ambiguous. Matching IDs: {}",
                    selector, ids
                ),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn provider(id: &str, name: &str) -> Provider {
        Provider::with_id(
            id.to_string(),
            name.to_string(),
            json!({
                "env": {
                    "ANTHROPIC_AUTH_TOKEN": format!("sk-{id}")
                }
            }),
            None,
        )
    }

    #[test]
    fn selector_prefers_exact_id_over_name() {
        let providers = IndexMap::from([
            ("demo".to_string(), provider("demo", "Shared Name")),
            ("other".to_string(), provider("other", "demo")),
        ]);

        let resolved = resolve_provider_selector(&providers, "demo").expect("resolve provider");

        assert_eq!(resolved.id, "demo");
    }

    #[test]
    fn selector_accepts_exact_name_when_id_is_missing() {
        let providers = IndexMap::from([("demo".to_string(), provider("demo", "Claude Demo"))]);

        let resolved =
            resolve_provider_selector(&providers, "Claude Demo").expect("resolve by name");

        assert_eq!(resolved.id, "demo");
    }

    #[test]
    fn selector_rejects_ambiguous_names() {
        let providers = IndexMap::from([
            ("demo-a".to_string(), provider("demo-a", "Shared Name")),
            ("demo-b".to_string(), provider("demo-b", "Shared Name")),
        ]);

        let err = resolve_provider_selector(&providers, "Shared Name")
            .expect_err("ambiguous name should fail");

        assert!(err.to_string().contains("Shared Name"));
        assert!(err.to_string().contains("demo-a"));
        assert!(err.to_string().contains("demo-b"));
    }

    #[test]
    fn start_claude_launches_resolved_provider() {
        let providers = IndexMap::from([("demo".to_string(), provider("demo", "Claude Demo"))]);
        let launched = std::cell::RefCell::new(None::<String>);

        start_claude_with(
            "Claude Demo",
            || Ok(providers),
            |provider| {
                launched.replace(Some(provider.id.clone()));
                Ok(())
            },
        )
        .expect("start command should launch matching provider");

        assert_eq!(launched.into_inner().as_deref(), Some("demo"));
    }
}
