use super::schemas;
use crate::engine;

use engine::api::AddOrUpdateOperation;
use engine::{job_board::JobTier, ErrorCategory, ErrorCode};
use std::sync::Arc;
use uuid::Uuid;
use warp::http::StatusCode;
use warp::{reject, reply, Rejection, Reply};

pub async fn add_user_to_registry(api: Arc<engine::Api>) -> Result<impl Reply, Rejection> {
    info!("Handling: add_user_to_registry");

    let user_id = uuid::Uuid::new_v4();
    match api.add_user(&user_id).await {
        Ok(_) => Ok(reply::with_status(
            reply::json(&user_id),
            StatusCode::CREATED,
        )),
        Err(e) => Err(reject::custom(EngineError {
            error: e,
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        })),
    }
}

pub async fn claim_daily_for_user(
    user_id: Uuid,
    api: Arc<engine::Api>,
) -> Result<impl Reply, Rejection> {
    info!("Handling: claim_daily_for_user");

    match api.claim_user_daily_reward(&user_id).await {
        Ok(currency) => Ok(reply::with_status(
            reply::json(&schemas::ClaimDailyForUserResponse { user_id, currency }),
            StatusCode::OK,
        )),
        Err(e) => {
            let status_code;
            if let ErrorCode::DailyAlreadyClaimed = e.code {
                status_code = StatusCode::CONFLICT;
            } else {
                status_code = get_http_code(&e);
            }

            Err(reject::custom(EngineError {
                error: e,
                status_code,
            }))
        }
    }
}

pub async fn confirm_staged_card(
    user_id: Uuid,
    api: Arc<engine::Api>,
    body: schemas::ConfirmStagedCardRequest,
) -> Result<impl Reply, Rejection> {
    info!("Handling: confirm_staged_card");

    match body.action {
        schemas::StagedCardAction::Promote => {
            match api.promote_staged_card(&user_id, &body.card_id).await {
                Ok(card) => Ok(reply::with_status(reply::json(&card), StatusCode::OK)),
                Err(e) => {
                    let status_code;
                    if let ErrorCode::CardNotFound = e.code {
                        status_code = StatusCode::INTERNAL_SERVER_ERROR;
                    } else {
                        status_code = get_http_code(&e);
                    }
                    Err(reject::custom(EngineError {
                        error: e,
                        status_code,
                    }))
                }
            }
        }
        schemas::StagedCardAction::Scrap => {
            match api.scrap_staged_card(&user_id, &body.card_id).await {
                Ok(currency) => Ok(reply::with_status(
                    reply::json(&schemas::ScrapCardResponse { user_id, currency }),
                    StatusCode::OK,
                )),
                Err(e) => {
                    let status_code;
                    if let ErrorCode::CardNotFound = e.code {
                        status_code = StatusCode::INTERNAL_SERVER_ERROR;
                    } else {
                        status_code = get_http_code(&e);
                    }
                    Err(reject::custom(EngineError {
                        error: e,
                        status_code,
                    }))
                }
            }
        }
    }
}

pub async fn delete_user_from_registry(
    user_id: Uuid,
    api: Arc<engine::Api>,
) -> Result<impl Reply, Rejection> {
    info!("Handling: delete_user_from_registry");

    match api.delete_user(&user_id).await {
        Ok(_) => Ok(reply::with_status(reply::reply(), StatusCode::OK)),
        Err(e) => {
            if let ErrorCode::UserNotFound = e.code {
                Err(reject::custom(EngineError {
                    error: e,
                    status_code: StatusCode::NOT_FOUND,
                }))
            } else {
                Err(reject::custom(EngineError::new(e)))
            }
        }
    }
}

pub async fn draw_card_to_stage_for_user(
    user_id: Uuid,
    api: Arc<engine::Api>,
) -> Result<impl Reply, Rejection> {
    info!("Handling: draw_card_to_stage_for_user");

    match api.draw_card(&user_id).await {
        Ok(currency) => Ok(reply::with_status(
            reply::json(&schemas::DrawCardToStageForUserResponse { user_id, currency }),
            StatusCode::OK,
        )),
        Err(e) => Err(reject::custom(EngineError::new(e))),
    }
}

pub async fn get_card_from_compendium(
    card_id: Uuid,
    api: Arc<engine::Api>,
) -> Result<impl Reply, Rejection> {
    info!("Handling: get_card_from_compendium");

    match api.get_card(&card_id).await {
        Ok(Some(card)) => Ok(reply::with_status(reply::json(&card), StatusCode::OK)),
        Ok(None) => Err(reject::not_found()),
        Err(e) => Err(reject::custom(EngineError {
            error: e,
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        })),
    }
}

pub async fn get_character_for_user(
    user_id: Uuid,
    character_id: Uuid,
    api: Arc<engine::Api>,
) -> Result<impl Reply, Rejection> {
    info!("Handling: get_character_for_user");

    match api.get_character(&user_id, &character_id).await {
        Ok(Some(character)) => Ok(reply::with_status(reply::json(&character), StatusCode::OK)),
        Ok(None) => Err(reject::not_found()),
        Err(e) => Err(reject::custom(EngineError {
            error: e,
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        })),
    }
}

pub async fn get_job_for_user(
    user_id: Uuid,
    job_id: Uuid,
    api: Arc<engine::Api>,
) -> Result<impl Reply, Rejection> {
    info!("Handling: get_job_for_user");

    match api.get_job(&user_id, &job_id).await {
        Ok(Some(job)) => Ok(reply::with_status(reply::json(&job), StatusCode::OK)),
        Ok(None) => Err(reject::not_found()),
        Err(e) => Err(reject::custom(EngineError {
            error: e,
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        })),
    }
}

pub async fn get_staged_card(
    user_id: Uuid,
    api: Arc<engine::Api>,
) -> Result<impl Reply, Rejection> {
    info!("Handling: get_staged_card");

    match api.get_staged_card(&user_id).await {
        Ok(Some(card)) => Ok(reply::with_status(reply::json(&card), StatusCode::OK)),
        Ok(None) => Err(reject::not_found()),
        Err(e) => {
            let status_code = match e.code {
                ErrorCode::CardNotFound => StatusCode::INTERNAL_SERVER_ERROR,
                ErrorCode::DrawStageEmpty => StatusCode::NOT_FOUND,
                _ => get_http_code(&e),
            };
            Err(reject::custom(EngineError {
                error: e,
                status_code,
            }))
        }
    }
}

pub async fn get_user_from_registry(
    user_id: Uuid,
    api: Arc<engine::Api>,
) -> Result<impl Reply, Rejection> {
    info!("Handling: get_user_from_registry");

    match api.get_user(&user_id).await {
        Ok(Some(user)) => Ok(reply::with_status(reply::json(&user), StatusCode::OK)),
        Ok(None) => Err(reject::not_found()),
        Err(e) => Err(reject::custom(EngineError {
            error: e,
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        })),
    }
}

pub async fn list_available_jobs(
    tier: String,
    api: Arc<engine::Api>,
) -> Result<impl Reply, Rejection> {
    info!("Handling: list_available_jobs");

    let job_tier = match tier.as_str() {
        "beginner" => Some(JobTier::Beginner),
        "intermediate" => Some(JobTier::Intermediate),
        "expert" => Some(JobTier::Expert),
        _ => None,
    };

    if let Some(tier) = job_tier {
        match api.list_available_jobs(&tier).await {
            Ok(jobs) => Ok(reply::with_status(reply::json(&jobs), StatusCode::OK)),
            Err(e) => Err(reject::custom(EngineError {
                error: e,
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })),
        }
    } else {
        Err(reject::not_found())
    }
}

pub async fn list_characters_for_user(
    user_id: Uuid,
    api: Arc<engine::Api>,
) -> Result<impl Reply, Rejection> {
    info!("Handling: list_characters_for_user");

    match api.list_characters(&user_id).await {
        Ok(characters) => Ok(reply::with_status(
            reply::json(&schemas::ListCharactersForUserResponse { characters }),
            StatusCode::OK,
        )),
        Err(e) => Err(reject::custom(EngineError::new(e))),
    }
}

pub async fn list_cards_from_compendium(api: Arc<engine::Api>) -> Result<impl Reply, Rejection> {
    info!("Handling: list_cards_from_compendium");

    match api.list_card_ids().await {
        Ok(card_ids) => Ok(reply::with_status(
            reply::json(&schemas::ListCardsFromCompendiumResponse::from(card_ids)),
            StatusCode::OK,
        )),
        Err(e) => Err(reject::custom(EngineError {
            error: e,
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        })),
    }
}

pub async fn list_jobs_for_user(
    user_id: Uuid,
    api: Arc<engine::Api>,
) -> Result<impl Reply, Rejection> {
    info!("Handling: list_jobs_for_user");

    match api.list_jobs(&user_id).await {
        Ok(jobs) => Ok(reply::with_status(reply::json(&jobs), StatusCode::OK)),
        Err(e) => Err(reject::custom(EngineError::new(e))),
    }
}

pub async fn list_users_from_registry(api: Arc<engine::Api>) -> Result<impl Reply, Rejection> {
    info!("Handling: list_users_from_registry");

    match api.list_user_ids().await {
        Ok(user_ids) => Ok(reply::with_status(
            reply::json(&schemas::ListUsersFromRegistryResponse::from(user_ids)),
            StatusCode::OK,
        )),
        Err(e) => Err(reject::custom(EngineError {
            error: e,
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        })),
    }
}

pub async fn put_card_to_compendium(
    card_id: Uuid,
    api: Arc<engine::Api>,
    body: schemas::PutCardToCompendiumRequest,
) -> Result<impl Reply, Rejection> {
    info!("Handling: put_card_to_compendium");

    // Validate explicit ID parameter matches ID in body
    if card_id != body.card.id {
        return Err(reject::custom(MessageError {
            error_message: "id mismatch".to_owned(),
            status_code: StatusCode::BAD_REQUEST,
        }));
    }

    match api.add_or_update_card_in_compendium(body.card).await {
        Ok(AddOrUpdateOperation::Add) => {
            Ok(reply::with_status(reply::reply(), StatusCode::CREATED))
        }
        Ok(AddOrUpdateOperation::Update) => Ok(reply::with_status(reply::reply(), StatusCode::OK)),
        Err(e) => Err(reject::custom(EngineError {
            error: e,
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        })),
    }
}

pub async fn recall_job_for_user(
    user_id: Uuid,
    job_id: Uuid,
    api: Arc<engine::Api>,
    body: schemas::RecallJobRequest,
) -> Result<impl Reply, Rejection> {
    info!("Handling: recall_job_for_user");

    match body.action {
        schemas::RecallJobAction::Cancel => {
            match api.cancel_job(&user_id, &job_id).await {
                Ok(_) => Ok(reply::with_status(reply::json(&()), StatusCode::OK)),
                Err(e) => Err(reject::custom(EngineError {
                    error: e,
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                })),
            }
        },
        schemas::RecallJobAction::Complete => {
            match api.complete_job(&user_id, &job_id).await {
                Ok(report) => Ok(reply::with_status(reply::json(&report), StatusCode::OK)),
                Err(e) => {
                    let status_code = match e.code {
                        ErrorCode::JobNotComplete => StatusCode::CONFLICT,
                        _ => get_http_code(&e),
                    };
                    Err(reject::custom(EngineError {
                        error: e,
                        status_code,
                    }))
                },
            }
        }
    }
}

pub async fn take_job_for_user(
    user_id: Uuid,
    api: Arc<engine::Api>,
    body: schemas::TakeJobRequest,
) -> Result<impl Reply, Rejection> {
    info!("Handling: take_job_for_user");

    // Validate at least 1 character id provided
    if body.character_ids.len() == 0 {
        return Err(reject::custom(MessageError {
            error_message: "character_ids cannot be empty".to_owned(),
            status_code: StatusCode::BAD_REQUEST,
        }));
    }

    match api
        .take_job(user_id, &body.job_prototype_id, body.character_ids)
        .await
    {
        Ok(job) => Ok(reply::with_status(reply::json(&job), StatusCode::OK)),
        Err(e) => Err(reject::custom(EngineError::new(e))),
    }
}

fn get_http_code(error: &engine::Error) -> http::StatusCode {
    match error.classify() {
        ErrorCategory::BadRequest => StatusCode::BAD_REQUEST,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

pub async fn handle_engine_error(err: Rejection) -> Result<impl Reply, Rejection> {
    if let Some(e) = err.find::<EngineError>() {
        let json = warp::reply::json(&e.error);
        Ok(warp::reply::with_status(json, e.status_code))
    } else if let Some(e) = err.find::<MessageError>() {
        let json = warp::reply::json(&ErrorResponse {
            error_message: e.error_message.clone(),
        });
        Ok(warp::reply::with_status(json, e.status_code))
    } else {
        Err(err)
    }
}

#[derive(Debug)]
struct EngineError {
    pub error: engine::Error,
    pub status_code: StatusCode,
}

impl EngineError {
    fn new(error: engine::Error) -> EngineError {
        let status_code = get_http_code(&error);
        EngineError { error, status_code }
    }
}

impl reject::Reject for EngineError {}

#[derive(Debug)]
struct MessageError {
    pub error_message: String,
    pub status_code: StatusCode,
}

impl reject::Reject for MessageError {}

#[derive(serde::Serialize)]
struct ErrorResponse {
    error_message: String,
}
