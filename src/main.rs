mod config;
mod util;
mod args;
mod commands;

use log::{error, info, warn};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::Path,
};

use actix_multipart::Multipart;
use actix_cors::Cors;
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
use clap::Parser;

use crate::config::{
    Config,
    UploadMode
};
use crate::util::*;
use crate::args::*;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let cli = Cli::parse();

    // 处理命令行参数
    match &cli.command {
        Some(Commands::Clear) => {
            commands::clear_storage();
            return Ok(());
        },
        None => {}
    };

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
    let max_file_size = config.max_file_size();
    info!("Max file size: {}", format_file_size(max_file_size));
    let use_token = config.use_token();
    info!("Use token: {}", use_token);
    let hashed_token = config.hashed_token();
    if use_token {
        info!("Hashed token: {}", hashed_token);
    }
    let upload_mode = config.upload_mode();
    info!("Upload mode: {:?}", upload_mode);
    let upload_whitelist = config.upload_whitelist();
    info!("Upload whitelist: {:?}", upload_whitelist);
    let upload_blacklist = config.upload_blacklist();
    info!("Upload blacklist: {:?}", upload_blacklist);

    let file_storage_path = format!("{}/file", www_root);
    if !Path::new(&file_storage_path).exists() {
        warn!("File storage path {} not exists, making it.", &file_storage_path);
        fs::create_dir(&file_storage_path).unwrap();
    }
    let total_size = calculate_total_size(&file_storage_path);
    let total_size_str = format_file_size(total_size as usize);
    info!("File storage total size: {}", total_size_str);
    let total_count = get_file_count(&file_storage_path);
    info!("File count: {}", total_count);

    let app_state = AppState {
        www_root,
        ssl,
        host,
        port,
        proxy,
        max_file_size,
        use_token,
        hashed_token,
        upload_mode,
        upload_whitelist,
        upload_blacklist,
    };

    let server = match HttpServer::new(move || {
        let cors = Cors::default().allow_any_origin();
        App::new()
            .wrap(cors)
            .app_data(web::Data::new(app_state.clone()))
            .service(index)
            .service(get_file)
            .service(upload_file)
            .service(delete_file)
    })
    .bind((listen_ip, port))
    {
        Ok(s) => {
            info!("Server established successfully");
            s
        }
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
    max_file_size: usize,
    use_token: bool,
    hashed_token: String,
    upload_mode: UploadMode,
    upload_whitelist: Vec<String>,
    upload_blacklist: Vec<String>,
}

#[get("/")]
async fn index(data: web::Data<AppState>) -> impl Responder {
    let www_root = &data.www_root;
    let host = &data.host;
    let port = data.port;
    let ssl = data.ssl;
    let proxy = data.proxy;
    let max_file_size = data.max_file_size;
    let max_file_size_str = format_file_size(max_file_size);
    let use_token = data.use_token;

    let protocol = match ssl {
        true => "https".to_string(),
        false => "http".to_string(),
    };
    let (request_url, delete_url) = match proxy {
        true => (
            format!("{}://{}/upload", protocol, host),
            format!("{}://{}/delete", protocol, host),
        ),
        false => (
            format!("{}://{}:{}/upload", protocol, host, port),
            format!("{}://{}:{}/delete", protocol, host, port),
        ),
    };

    let index_path = format!("{}/index.html", www_root);
    let file_storage_path = format!("{}/file", www_root);
    let total_size = calculate_total_size(&file_storage_path);
    let total_size_str = format_file_size(total_size as usize);
    let total_count = get_file_count(&file_storage_path);
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
    let replacements = [
        ("UPLOAD", &request_url),
        ("TOTAL_SIZE", &total_size_str),
        ("MAX_SIZE", &max_file_size_str),
        ("TOTAL_COUNT", &total_count.to_string()),
        ("DELETE", &delete_url),
        ("USE_TOKEN", &(use_token.to_string())),
    ];

    for (pattern, replacement) in replacements.iter() {
        index_content = index_content.replace(pattern, replacement);
    }

    info!("Request for index is OK");

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(index_content)
}

// 存在于白名单中的文件将被认为存放在www_root下，而不是www_root/file下
const FILE_WHITELIST: [&'static str; 2] = ["favicon.ico", "style.css", "CircularBody.woff"];

#[get("/{filename}")]
async fn get_file(data: web::Data<AppState>, filename: web::Path<String>) -> impl Responder {
    let www_root = &data.www_root;
    let file_path = if FILE_WHITELIST.contains(&filename.as_str()) {
        format!("{}/{}", www_root, filename)
    } else {
        format!("{}/file/{}", www_root, filename)
    };
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

            warn!("File {} not found when trying to access it.", &filename);

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
    let ssl = data.ssl;
    let host = &data.host;
    let port = data.port;
    let proxy = data.proxy;
    let max_file_size = data.max_file_size;
    let max_file_size_str = format_file_size(max_file_size);
    let use_token = data.use_token;
    let hashed_token = &data.hashed_token;
    let upload_mode = &data.upload_mode;
    let upload_whitelist = &data.upload_whitelist;
    let upload_blacklist = &data.upload_blacklist;

    // 生成一个唯一的文件名（基于文件内容的哈希值）
    let mut hasher = Sha256::new();
    let mut file_content = Vec::new();

    // 先接收token
    if use_token {
        match payload.next().await.unwrap() {
            Ok(mut field) => {
                let mut token_chunk = Vec::new();
                while let Some(chunk) = field.next().await {
                    token_chunk.extend_from_slice(&chunk.unwrap());
                }
                let token = String::from_utf8(token_chunk).unwrap();
                if get_str_sha256(&token) != *hashed_token {
                    return HttpResponse::BadRequest().body("Incorrect token!");
                }
            }
            Err(_) => return HttpResponse::InternalServerError().finish(),
        }
    }

    let file_extension;

    // 然后接收文件
    match payload.next().await.unwrap() {
        Ok(mut field) => {
            let cd = field.content_disposition();
            let file_name = cd.get_filename().unwrap_or("unknown");

            // 获取文件扩展名
            file_extension = Path::new(file_name)
                .extension()
                .and_then(std::ffi::OsStr::to_str)
                .unwrap_or("unknown")
                .to_string();
            // 检查上传模式
            if *upload_mode == UploadMode::Whitelist {
                if !upload_whitelist.contains(&file_extension) {
                    error!("File extension {} not in whitelist.", &file_extension);
                    return HttpResponse::BadRequest().body(format!(
                        "You can't upload a file with extension {}, because the extension is not in whitelist.",
                        &file_extension
                    ));
                }
            } else if *upload_mode == UploadMode::Blacklist {
                if upload_blacklist.contains(&file_extension) {
                    error!("File extension {} in blacklist.", &file_extension);
                    return HttpResponse::BadRequest().body(format!(
                        "You can't upload a file with extension {}, because the extension is in blacklist.",
                        &file_extension
                    ));
                }
            }

            while let Some(chunk) = field.next().await {
                file_content.extend_from_slice(&chunk.unwrap());
                let file_size = file_content.len();
                if file_size > max_file_size {
                    error!("The file size is too large, refused.");
                    let file_size_str = format_file_size(file_size);
                    return HttpResponse::BadRequest().body(format!(
                        "The file is too large (already got {}, expected less than {}).",
                        &file_size_str, &max_file_size_str
                    ));
                }
                hasher.update(&file_content);
            }
            let file_size = file_content.len();
            let file_size_str = format_file_size(file_size);
            info!("The file size is {}", &file_size_str);
        }
        Err(_) => return HttpResponse::InternalServerError().finish(),
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

    if filename.is_empty() {
        return HttpResponse::BadRequest().body("Please do not send blank file name");
    }

    let path = format!("{}/file/{}", www_root, filename);

    if Path::new(&path).exists() {
        match fs::remove_file(&path) {
            Ok(_) => {
                info!("File {} deleted.", &filename);
                return HttpResponse::Ok().body(format!("{} deleted", filename));
            }
            Err(err) => {
                error!("Internal error when deleting file {}.", &filename);
                return HttpResponse::InternalServerError()
                    .body(format!("Error deleting file {}: {:?}", filename, err));
            }
        }
    } else {
        warn!("File {} not fount when trying to delete it.", &filename);
        return HttpResponse::NotFound().body(format!("{} not found", filename));
    }
}
