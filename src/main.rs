use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder, FromRequest};
use serde::{Serialize, Deserialize};
use tokio::{fs::File, io::AsyncWriteExt};
use tokio::io::AsyncReadExt;
use serde_json;
use serde_json::Value;
use std::collections::HashMap;
use chrono::prelude::*;
use serde_json::json;
use serde_json::from_str;
use reqwest::header::{HeaderMap, HeaderValue};


#[derive(Deserialize, Serialize)]
struct FormData {
    id: Option<String>,
    base64: Option<String>,
    imgUrl: Option<String>,
    devId: Option<String>,
    devName: Option<String>,
    name: Option<String>,
    devVol: Option<String>,
    csq: Option<String>,
    forwardType: Option<String>,
    createTime: Option<String>,
    isPointerMeter: Option<String>,
    isMultiRegionMeter: Option<String>,
    result: Option<String>
}

#[derive(Serialize, Deserialize)]
struct State {
    ts: i64,
    val: f64,
}

async fn write_state(state: &State) -> std::io::Result<()> {
    let mut file = File::create("current_state.dat").await?;
    let serialized = serde_json::to_string(state)?;
    file.write_all(serialized.as_bytes()).await?;
    Ok(())
}

async fn read_state() -> std::io::Result<State> {
    let mut file = File::open("current_state.dat").await?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).await?;
    let deserialized: State = serde_json::from_slice(&buffer)?;
    Ok(deserialized)
}

#[get("/")]
async fn main_func() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

#[post("/")]
async fn post_func(form: web::Form<FormData>) -> impl Responder {
    let now = Utc::now();
    let ts: i64 = now.timestamp();
    let file_name = format!("logs/{:?}.log", ts);
    let json_str = serde_json::to_string(&form).unwrap();
    let json_bytes = json_str.as_bytes();
    let mut file = File::create(file_name.as_str()).await.unwrap();
    file.write_all(json_bytes).await.unwrap();
    println!("写入完成");
    let ct = match form.createTime.clone() {
        Some(v)=>v,
        _=>"".to_string()
    };
    let trans_ct = ct.replace(" ", "T");
    let value = match form.result.clone() {
        Some(v)=>v,
        _=>"{}".to_string()
    };
    let parsed: HashMap<String, Value> = from_str(value.as_str()).unwrap();
    println!("Got result:{:?}", &parsed);
    let value:i64 = parsed.get("outputState").unwrap().as_i64().unwrap();
    if value < 1{
        let integer_val: &str = parsed.get("outputStrInt").unwrap().as_str().unwrap();
        let decimal_val: &str = parsed.get("outputStrDec").unwrap().as_str().unwrap();
        let str_val = format!("{}.{}", integer_val, decimal_val);
        println!("meter value:{}", str_val);
        let float_val: f64 = str_val.parse().unwrap();

        let state: State = match read_state().await {
            Ok(state) => state,
            Err(e) => State{ts:0, val:0.0}
        };

        let val_diff = if state.ts > 0 { float_val - state.val } else { 0.0 };
        let ts_diff = if state.ts > 0 { ts - state.ts } else { 0 };
        let c2 = if state.ts > 0 { val_diff/(ts_diff as f64/(3600.0 * 1000.0) as f64) } else {0.0};

        let url = format!("http://api.heclouds.com/devices/{}/datapoints", "507319845");
        let client = reqwest::Client::new();
        let mut headers = HeaderMap::new();
        headers.insert("api-key", HeaderValue::from_static("4WaD2dLGgzp9zgWX3HIUO04NUCQ="));
        headers.insert("content-type", HeaderValue::from_static("application/json"));
        let json_data = json!({
      "datastreams": [
        {
            "id": "C1",
          "datapoints": [{
                "at": trans_ct.as_str(),
                  "value": float_val
            },
          ]
        },
        {
            "id": "C2",
          "datapoints": [{
                "at": trans_ct.as_str(),
                  "value": c2
            },
          ]
        }
      ]
        });
        println!("will post json:{:?}", json_data.clone());
        let res = client.post(url)
        .headers(headers)
        .json(&json_data)
        .send()
        .await.expect("connect fail");
        println!("{}", res.text().await.expect("read fail"));
    }
    HttpResponse::Ok().body("Ok!")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .data(web::JsonConfig::default().limit( 10 * 1024 * 1024))
            .data(web::FormConfig::default().limit( 10 * 1024 * 1024))
            .service(main_func)
            .service(post_func)
    })
    .bind(("0.0.0.0", 8000))?
    .run()
    .await
}

