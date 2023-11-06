mod config;

use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
    time::SystemTime,
};

use futures_util::stream::StreamExt;
use actix_multipart::Multipart;
use actix_web::{
    get,
    http::header::ContentType,
    post,
    web::{self, Bytes},
    App, HttpResponse, HttpServer, Responder,
};
use new_mime_guess;
use sha2::{Sha256, Digest};

use crate::config::Config;

#[derive(Clone, Debug)]
struct AppState {
    www_root: String,
    ssl: bool,
    host: String,
    port: u16,
    proxy: bool,
}

/// # get_time
/// 
/// 用于获取系统时间。
/// 
/// 返回的是从 1970-01-01 00:00:00 UTC 起到现在的秒数。
fn get_time() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs()
}

fn shorten(input: &str) -> String {
    assert_eq!(input.len(), 64);
    let (left_half, right_half) = input.split_at(32);

    let left = u128::from_str_radix(left_half, 16).unwrap();
    let right = u128::from_str_radix(right_half, 16).unwrap();
    let result = left.wrapping_add(right);

    format!("{:x}", result)
}

#[get("/")]
async fn index(data: web::Data<AppState>) -> impl Responder {
    let www_root = &data.www_root;
    let host = &data.host;
    let port = &data.port;
    let ssl = &data.ssl;
    let proxy = &data.proxy;

    let protocol = match ssl {
        true => "https".to_string(),
        false => "http".to_string(),
    };
    let request_url = match proxy {
        true => format!("{}://{}/upload", protocol, host),
        false => format!("{}://{}:{}/upload", protocol, host, port),
    };

    let index_path = format!("{}/index.html", www_root);
    let mut index_file = File::open(&index_path)
        .map_err(|e| {
            eprintln!("Couldn't open index.html: {}", e);
            HttpResponse::InternalServerError().finish()
        })
        .unwrap();
    let mut index_content = String::new();
    index_file
        .read_to_string(&mut index_content)
        .expect("Couldn't read index.html!");
    let index_content = index_content.replace("UPLOAD", &request_url);

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(index_content)
}

#[get("/{filename}")]
async fn get_file(data: web::Data<AppState>, filename: web::Path<String>) -> impl Responder {
    let www_root = &data.www_root;
    let file_path = format!("{}/file/{}", www_root, filename);
    let mut file = match File::open(&file_path) {
        Ok(f) => f,
        Err(_) => {
            // 文件不存在，返回404错误，使用404.html作为响应
            let not_found_path = format!("{}/404.html", www_root);
            let not_found_content = match File::open(&not_found_path) {
                Ok(mut file) => {
                    let mut content = Vec::new();
                    file.read_to_end(&mut content).unwrap();
                    content
                }
                Err(_) => "<h1>404 Not Found</h1>".as_bytes().to_vec(),
            };

            return HttpResponse::NotFound()
                .content_type("text/html; charset=utf-8")
                .body(Bytes::from(not_found_content));
        }
    };
    let mut content = Vec::new();

    // 以字节数组的形式读取文件内容
    let file_content = web::block(move || {
        file.read_to_end(&mut content).unwrap();
        content
    })
    .await
    .map_err(|e| {
        eprintln!("Error reading file: {:?}", e);
        HttpResponse::InternalServerError().finish()
    })
    .unwrap();

    let guess = new_mime_guess::from_path(filename.as_str())
        .first()
        .unwrap();

    HttpResponse::Ok()
        .insert_header(ContentType(guess))
        .body(Bytes::from(file_content))
}

#[post("/upload")]
async fn upload_file(data: web::Data<AppState>, mut payload: Multipart) -> impl Responder {
    let www_root = &data.www_root;
    let ssl = &data.ssl;
    let host = &data.host;
    let port = &data.port;
    let proxy = &data.proxy;

    // 生成一个唯一的文件名（基于文件内容的哈希值）
    let mut hasher = Sha256::new();
    let mut file_content = Vec::new();
    let mut file_extension = String::new();
    while let Some(item) = payload.next().await {
        match item {
            Ok(mut field) => {
                let content_type = field.content_disposition();
                let file_name = content_type.get_filename().unwrap_or("unknown");
                file_extension = Path::new(file_name)
                    .extension()
                    .and_then(std::ffi::OsStr::to_str)
                    .unwrap_or("unknown")
                    .to_string();
                while let Some(chunk) = field.next().await {
                    file_content.extend_from_slice(&chunk.unwrap());
                    hasher.update(&file_content);
                }
            },
            Err(_) => return HttpResponse::InternalServerError().finish(),
        }
    }

    hasher.update(get_time().to_string().as_bytes());

    let file_hash = hasher.finalize();
    let file_hash_str = format!("{:x}", file_hash);
    let shortened_file_hash_str = shorten(&file_hash_str);

    // 构建文件保存路径
    let file_name = format!("{}.{}", shortened_file_hash_str, file_extension);
    let file_path = format!("{}/file/{}", www_root, file_name);
    // 保存文件
    let mut file = web::block(move || {
        File::create(&file_path).map_err(|e| {
            eprintln!("Error creating file: {:?}", e);
            HttpResponse::InternalServerError().finish()
        }).unwrap()
    })
    .await
    .unwrap();

    match file.write_all(file_content.as_slice()) {
        Ok(_) => (),
        Err(e) => {
            eprintln!("Error writing file: {:?}", e);
            return HttpResponse::InternalServerError().finish();
        }
    }

    // 返回文件的独一无二的URL（使用哈希值）
    let protocol = match ssl {
        true => "https".to_string(),
        false => "http".to_string(),
    };
    let file_url = match proxy {
        true => format!("{}://{}/{}", protocol, host, file_name),
        false => format!("{}://{}:{}/{}", protocol, host, port, file_name),
    };

    HttpResponse::Ok()
        .body(file_url)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // 加载配置文件
    let config = Config::from_toml("./config.toml");
    let www_root = config.www_root().to_string();
    let ssl = config.ssl();
    let host = config.host().to_string();
    let proxy = config.proxy();
    let port = config.port();
    let listen_ip = match config.local() {
        true => "127.0.0.1".to_string(),
        false => "0.0.0.0".to_string(),
    };

    let app_state = AppState { www_root, ssl, host, port, proxy };

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(app_state.clone()))
            .service(index)
            .service(get_file)
            .service(upload_file)
    })
    .bind((listen_ip, port))?
    .run()
    .await
}
