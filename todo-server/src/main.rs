use anyhow::Result;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{sqlite::SqlitePool, Sqlite, Pool};
use std::env;
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    let pool = SqlitePool::connect(&env::var("DATABASE_URL")?).await?;
    sqlx::migrate!().run(&pool).await?;

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "todo_server=debug,tower_http=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let app = app(pool);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
    Ok(())
}

fn app(pool: SqlitePool) -> Router<Pool<Sqlite>> {
    Router::with_state(pool)
            .route("/lists", get(handle_get_lists))
            .route("/lists/:id/todos", get(handle_get_todos))
            .layer(TraceLayer::new_for_http())
}

async fn handle_get_lists(State(pool): State<SqlitePool>) -> Result<impl IntoResponse, AppError> {
    let lists = get_lists(&pool).await?;
    Ok(Json(lists))
}

async fn handle_get_todos(
    State(pool): State<SqlitePool>,
    Path(list_id): Path<i32>,
) -> Result<impl IntoResponse, AppError> {
    let todos = get_todos(&pool, list_id).await?;
    Ok(Json(todos))
}

#[derive(Debug, Serialize, Deserialize)]
struct List {
    id: i64,
    name: String,
}

async fn get_lists(pool: &SqlitePool) -> Result<Vec<List>> {
    let result = sqlx::query_as!(
        List,
        r#"
SELECT id, name
FROM lists
ORDER BY id
        "#
    )
    .fetch_all(pool)
    .await?;

    Ok(result)
}

#[derive(Debug, Serialize, Deserialize)]
struct Todo {
    id: i64,
    text: String,
    checked: bool,
    list_id: i64,
}

async fn get_todos(pool: &SqlitePool, list_id: i32) -> Result<Vec<Todo>> {
    let result = sqlx::query_as!(
        Todo,
        r#"
SELECT id, text, checked, list_id
FROM todos
WHERE list_id = ?
ORDER BY id
        "#,
        list_id
    )
    .fetch_all(pool)
    .await?;

    Ok(result)
}

enum AppError {
    InternalServerError(anyhow::Error),
}

impl From<anyhow::Error> for AppError {
    fn from(inner: anyhow::Error) -> Self {
        AppError::InternalServerError(inner)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::InternalServerError(_inner) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong")
            }
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use serde_json::{json, Value};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_lists() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();

        let app = app(pool);

        let response = app
            .oneshot(Request::builder().uri("/lists").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body, json!([]));
    }
}