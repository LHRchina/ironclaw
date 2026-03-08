//! LLM preset management handlers.

use std::sync::Arc;

use axum::{
    Json,
    extract::State,
    http::StatusCode,
};

use crate::channels::web::llm_presets;
use crate::channels::web::server::GatewayState;
use crate::channels::web::types::{
    LlmPresetInfo, LlmPresetListResponse, LlmPresetSelectRequest, LlmPresetSelectResponse,
};

pub async fn llm_presets_list_handler(
    State(_state): State<Arc<GatewayState>>,
) -> Result<Json<LlmPresetListResponse>, (StatusCode, String)> {
    let listed = llm_presets::list_llm_presets()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(LlmPresetListResponse {
        source_path: listed.source_path.display().to_string(),
        active_label: listed.active_label,
        presets: listed
            .presets
            .into_iter()
            .map(|preset| LlmPresetInfo {
                label: preset.label,
                backend: preset.backend,
                model: preset.model,
                base_url: preset.base_url,
                active: preset.active,
            })
            .collect(),
    }))
}

pub async fn llm_presets_select_handler(
    State(_state): State<Arc<GatewayState>>,
    Json(body): Json<LlmPresetSelectRequest>,
) -> Result<Json<LlmPresetSelectResponse>, (StatusCode, String)> {
    let selection = llm_presets::select_llm_preset(&body.label).map_err(|e| {
        let status = match e {
            llm_presets::LlmPresetError::UnknownPreset(_) => StatusCode::NOT_FOUND,
            llm_presets::LlmPresetError::EnvFileNotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, e.to_string())
    })?;

    let message = if selection.changed {
        format!(
            "Saved '{}' to {}. Restart IronClaw to apply the new LLM.",
            selection.active_label,
            selection.source_path.display()
        )
    } else {
        format!(
            "'{}' is already active in {}.",
            selection.active_label,
            selection.source_path.display()
        )
    };

    Ok(Json(LlmPresetSelectResponse {
        active_label: selection.active_label,
        source_path: selection.source_path.display().to_string(),
        changed: selection.changed,
        restart_required: selection.changed,
        message,
    }))
}
