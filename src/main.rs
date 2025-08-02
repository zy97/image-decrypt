use std::{
    collections::HashMap,
    fs,
    sync::{Arc, Mutex},
    time::Duration,
};

use actix_web::{App, HttpRequest, HttpResponse, HttpServer, Responder, get, web};
use base64::{Engine as _, decode, engine::general_purpose};
use deno_core::{JsRuntime, RuntimeOptions, url, v8::Local};
use moka::future::Cache;
use reqwest;
use serde_v8::from_v8;

const ONE_WEEK_IN_SECONDS: u64 = 60 * 60 * 24 * 7;
async fn cached_endpoint(key: String, cache: web::Data<Cache<String, String>>) -> Option<String> {
    // 尝试从缓存获取
    cache.get(&key).await
}
async fn get_or_insert<F, Fut>(
    key: String,
    cache: web::Data<Cache<String, String>>,
    setter: F,
) -> Option<String>
where
    F: Fn(String) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = String> + Send + 'static,
{
    // 尝试从缓存获取
    if let Some(data) = cache.get(&key).await {
        return Some(data);
    }

    // 如果缓存中没有数据，调用异步 setter 函数生成数据并插入缓存
    let value = setter(key.clone()).await;
    cache.insert(key, value.clone()).await;
    Some(value)
}

#[get("/")]
async fn greet1(req: HttpRequest, cache: web::Data<Cache<String, String>>) -> impl Responder {
    let query = req.query_string(); // 原始 query: "name=Tom&age=18"
    let params: std::collections::HashMap<_, _> = url::form_urlencoded::parse(query.as_bytes())
        .into_owned()
        .collect();

    let image_url = params.get("image").unwrap();
    let cache_data = cached_endpoint(image_url.to_string(), cache.clone()).await;
    match cache_data {
        Some(cached_image) => {
            match decode(cached_image) {
                Ok(image_bytes) => {
                    HttpResponse::Ok()
                        .content_type("image/png") // 或者 image/jpeg 等
                        .body(image_bytes)
                }
                Err(_) => HttpResponse::BadRequest().body("Invalid base64 string"),
            }
        }
        None => {
            // 获取图片内容
            let response = reqwest::get(image_url)
                .await
                .expect("Failed to fetch image");
            let image_data = response.bytes().await.expect("Failed to read image data");
            let image_data = image_data.to_vec();
            let image_data = general_purpose::STANDARD.encode(&image_data);
            let decrypt = fs::read_to_string("./decrypt.js").expect("Failed to read JS file");
            let crypto = fs::read_to_string("./crypto-js.js").expect("Failed to read JS file");
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
            // // 使用 JS 脚本中的函数，比如 CryptoJS.MD5
            let result = runtime
                .execute_script("test", execute_js)
                .expect("Failed to run script");

            let result_str = {
                let scope = &mut runtime.handle_scope(); // 创建作用域
                let local = Local::new(scope, &result);
                let value: String = from_v8(scope, local) // 转为 Rust 的 String
                    .expect("Failed to convert result");
                value
            };
            cache.insert(image_url.clone(), result_str.clone()).await;
            match decode(result_str) {
                Ok(image_bytes) => {
                    HttpResponse::Ok()
                        .content_type("image/png") // 或者 image/jpeg 等
                        .body(image_bytes)
                }
                Err(_) => HttpResponse::BadRequest().body("Invalid base64 string"),
            }
        }
    }
}
#[get("/")]
async fn greet(req: HttpRequest, cache: web::Data<Cache<String, String>>) -> impl Responder {
    let query = req.query_string(); // 原始 query: "name=Tom&age=18"
    let params: std::collections::HashMap<_, _> = url::form_urlencoded::parse(query.as_bytes())
        .into_owned()
        .collect();

    let image_url = params.get("image").unwrap().clone();
    let data = get_or_insert(image_url.clone(), cache, async move |key| {
        // 获取图片内容
        let response = reqwest::get(key).await.expect("Failed to fetch image");
        let image_data = response.bytes().await.expect("Failed to read image data");
        let image_data = image_data.to_vec();
        let image_data = general_purpose::STANDARD.encode(&image_data);
        let decrypt = fs::read_to_string("./decrypt.js").expect("Failed to read JS file");
        let crypto = fs::read_to_string("./crypto-js.js").expect("Failed to read JS file");
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
        // // 使用 JS 脚本中的函数，比如 CryptoJS.MD5
        let result = runtime
            .execute_script("test", execute_js)
            .expect("Failed to run script");

        let result_str = {
            let scope = &mut runtime.handle_scope(); // 创建作用域
            let local = Local::new(scope, &result);
            let value: String = from_v8(scope, local) // 转为 Rust 的 String
                .expect("Failed to convert result");
            value
        };
        result_str
    })
    .await;
    match data {
        Some(cached_image) => {
            match decode(cached_image) {
                Ok(image_bytes) => {
                    HttpResponse::Ok()
                        .content_type("image/png") // 或者 image/jpeg 等
                        .body(image_bytes)
                }
                Err(_) => HttpResponse::BadRequest().body("Invalid base64 string"),
            }
        }
        None => {
            return HttpResponse::NotFound().body("Image not found in cache or failed to fetch");
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // 创建缓存，设置一周有效期
    let cache: Cache<String, String> = Cache::builder()
        .time_to_live(Duration::from_secs(ONE_WEEK_IN_SECONDS))
        .build();
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(cache.clone()))
            .service(greet)
    })
    .bind(("0.0.0.0", 9090))?
    .run()
    .await
}
