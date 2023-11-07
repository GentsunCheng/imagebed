mod config;
mod util;

use std::{
    fs::{self, File},
    io::{Read, Write},
    path::Path,
};
use log::{error, info};

use actix_multipart::Multipart;
use actix_web::{
    get,
    http::header::ContentType,
    post,
    web::{self, Bytes},
    App, HttpResponse, HttpServer, Responder,
};
use futures_util::stream::StreamExt;
use log4rs;
use new_mime_guess;
use serde_derive::Deserialize;
use sha2::{Digest, Sha256};

use crate::config::Config;
use crate::util::*;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // 初始化日志系统
    log4rs::init_file("config/log4rs.yaml", Default::default()).unwrap();

    // 加载配置文件
    let config = Config::from_toml("config/config.toml");
    let www_root = config.www_root().to_string();
    info!("www_root: {}", &www_root);
    let ssl = config.ssl();
    info!("Use SSL: {}", ssl);
    let host = config.host().to_string();
    info!("Host: {}", &host);
    let proxy = config.proxy();
    info!("Use reverse proxy: {}", proxy);
    let port = config.port();
    info!("Listen port: {}", port);
    let listen_ip = match config.local() {
        true => "127.0.0.1".to_string(),
        false => "0.0.0.0".to_string(),
    };
    info!("Listen IP: {}", &listen_ip);

    let app_state = AppState {
        www_root,
        ssl,
        host,
        port,
        proxy,
    };

    let server = match HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(app_state.clone()))
            .service(index)
            .service(get_file)
            .service(upload_file)
            .service(delete_file)
    }).bind((listen_ip, port)) {
        Ok(s) => {
            info!("Server established successfully");
            s
        },
        Err(e) => {
            error!("Error happened when establishing server: {}", e);
            panic!();
        }
    };

    server.run().await
}

#[derive(Clone, Debug)]
struct AppState {
    www_root: String,
    ssl: bool,
    host: String,
    port: u16,
    proxy: bool,
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

    info!("Request for / OK");

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

            info!("File {} not fount when trying to access it.", &filename);

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

    info!("Request for {} OK. MIME is {}.", &filename, &guess);

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
            }
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
        File::create(&file_path)
            .map_err(|e| {
                eprintln!("Error creating file: {:?}", e);
                HttpResponse::InternalServerError().finish()
            })
            .unwrap()
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

    // 返回URL（使用哈希值）
    let protocol = match ssl {
        true => "https".to_string(),
        false => "http".to_string(),
    };
    let file_url = match proxy {
        true => format!("{}://{}/{}", protocol, host, file_name),
        false => format!("{}://{}:{}/{}", protocol, host, port, file_name),
    };

    info!("Upload file {} saved. URL is {}.", &file_name, &file_url);

    HttpResponse::Ok().body(file_url)
}

#[derive(Deserialize)]
struct DeleteRequest {
    file: String,
}

#[post("/delete")]
async fn delete_file(
    data: web::Data<AppState>,
    req_body: web::Json<DeleteRequest>,
) -> impl Responder {
    let www_root = &data.www_root;
    let filename = &req_body.file;

    let path = format!("{}/file/{}", www_root, filename);

    if Path::new(&path).exists() {
        match fs::remove_file(&path) {
            Ok(_) => {
                info!("File {} deleted.", &filename);
                return HttpResponse::Ok().body(format!("{} deleted", filename));
            }
            Err(err) => {
                info!("Internal error when deleting file {}.", &filename);
                return HttpResponse::InternalServerError()
                    .body(format!("Error deleting file {}: {:?}", filename, err));
            }
        }
    } else {
        info!("File {} not fount when trying to delete it.", &filename);
        return HttpResponse::NotFound().body(format!("{} not found", filename));
    }
}
