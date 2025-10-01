use axum::http::{HeaderMap, HeaderName};
use axum::response::IntoResponse;
use axum::{
    Router,
    extract::{Query, State},
    routing::get,
};
use base64::{Engine, engine::general_purpose};
use bb8_redis::{
    RedisConnectionManager,
    bb8::{self, Pool},
    redis::AsyncCommands,
};
use deno_core::{JsRuntime, RuntimeOptions, v8::Local};
use reqwest::header::CONTENT_TYPE;
use reqwest::{self, StatusCode};
use serde::Deserialize;
use serde_v8::from_v8;
use std::fs;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

async fn greet(query: Query<QueryInfo>, State(pool): State<ConnectionPool>) -> impl IntoResponse {
    let mut conn = pool.get().await.map_err(internal_error)?;

    match query.0.image {
        Some(image) => {
            let image_cache: Option<String> = conn.get(&image).await.map_err(internal_error)?;
            let image_base64 = match image_cache {
                Some(image) => Some(image),
                None => {
                    // 获取图片内容
                    let response = reqwest::get(&image).await.expect("Failed to fetch image");

                    let image_data = response.bytes().await.expect("Failed to read image data");
                    let image_data = image_data.to_vec();
                    let image_data = general_purpose::STANDARD.encode(&image_data);
                    let decrypt =
                        fs::read_to_string("./decrypt.js").expect("Failed to read JS file");
                    let crypto =
                        fs::read_to_string("./crypto-js.js").expect("Failed to read JS file");

                    let image_base64 = {
                        // 创建 JS 运行时
                        let mut runtime = JsRuntime::new(RuntimeOptions::default());

                        // 加载 crypto-js
                        runtime.execute_script("crypto-js", crypto).unwrap();
                        runtime.execute_script("decrypt", decrypt).unwrap();

                        let execute_js = format!(
                            r#"
                            decryptImage("{}");
                        "#,
                            image_data
                        );
                        let result = runtime
                            .execute_script("test", execute_js)
                            .expect("Failed to run script");

                        let image_base64 = {
                            let scope = &mut runtime.handle_scope(); // 创建作用域
                            let local = Local::new(scope, &result);
                            let value: String =
                            from_v8(scope, local) // 转为 Rust 的 String
                                .expect("Failed to convert result");
                            value
                        };
                        image_base64
                    };

                    conn.set::<&String, String, ()>(&image, image_base64.clone())
                        .await
                        .unwrap();
                    Some(image_base64)
                }
            };
            match image_base64 {
                Some(image) => {
                    let image_bytes = general_purpose::STANDARD.decode(image);
                    match image_bytes {
                        Ok(bytes) => {
                            let mut headers = HeaderMap::new();
                            headers.insert(CONTENT_TYPE, "image/png".parse().unwrap());
                            return Ok((StatusCode::OK, headers, bytes));
                        }
                        Err(_) => {
                            return Err((StatusCode::BAD_REQUEST, String::from("error image")));
                        }
                    }
                }
                None => return Err((StatusCode::BAD_REQUEST, String::from("no image"))),
            }
        }
        None => {
            return Err((StatusCode::BAD_REQUEST, String::from("no image")));
        }
    }

    // return (StatusCode::BAD_REQUEST, String::new());
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::debug!("connecting to redis");
    let manager = RedisConnectionManager::new("redis://localhost").unwrap();
    let pool = bb8::Pool::builder().build(manager).await.unwrap();

    {
        // ping the database before starting
        let mut conn = pool.get().await.unwrap();
        conn.set::<&str, &str, ()>("foo", "bar").await.unwrap();
        let result: String = conn.get("foo").await.unwrap();
        assert_eq!(result, "bar");
    }
    tracing::debug!("successfully connected to redis and pinged it");

    // build our application with a route
    let app = Router::new().route("/", get(greet)).with_state(pool);

    // run it
    let listener = tokio::net::TcpListener::bind("0.0.0.0:9090").await.unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
#[derive(Deserialize)]
struct QueryInfo {
    image: Option<String>,
}

type ConnectionPool = Pool<RedisConnectionManager>;

fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
