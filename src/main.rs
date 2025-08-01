use std::{fs, path::Path};

use actix_web::{get, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use base64::{decode, engine::general_purpose, Engine as _};
use deno_core::{url, v8::Local, JsRuntime, RuntimeOptions};
use reqwest;
use serde_v8::from_v8;
#[get("/")]
async fn greet(req: HttpRequest) -> impl Responder {
     let query = req.query_string(); // 原始 query: "name=Tom&age=18"
    let params: std::collections::HashMap<_, _> = url::form_urlencoded::parse(query.as_bytes()).into_owned().collect();

    let image_url = params.get("image").unwrap();
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
// result_str 是base64图片,现在转成图片返回
let image = 
    // println!("MD5 result: {:?}", result_str);
    //把图片转成string
    // 返回结果
     format!("{:#?}!", result_str);
     match decode(result_str) {
        Ok(image_bytes) => {
            HttpResponse::Ok()
                .content_type("image/png") // 或者 image/jpeg 等
                .body(image_bytes)
        }
        Err(_) => HttpResponse::BadRequest().body("Invalid base64 string"),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().service(greet))
        .bind(("127.0.0.1", 9090))?
        .run()
        .await
}
