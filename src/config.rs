use std::fs::File;
use std::io::prelude::*;

use serde_derive::Deserialize;
use serde_derive::Serialize;

use crate::util::get_str_sha256;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum UploadMode {
    None,
    Whitelist,
    Blacklist,
}

/// # Config
/// 
/// 存储服务配置信息
/// 
/// - `www_root`: 服务器的Web根路径
/// - `port`: 要绑定的本机端口
/// - `local`: 是否工作在内网。
///     - 如果设置为`true`，则监听IP是`127.0.0.1`
///     - 如果设置为`false`，则监听IP是`0.0.0.0`
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    www_root: String,
    proxy: bool,
    ssl: bool,
    host: String,
    port: u16,
    local: bool,
    max_file_size: usize,
    use_token: bool,
    token: String,
    upload_mode: UploadMode,
    upload_whitelist: Vec<String>,
    upload_blacklist: Vec<String>,
}

impl Config {
    /// 产生一个默认的`Config`对象
    pub fn new() -> Self {
        Self {
            www_root: ".".to_string(),
            proxy: false,
            ssl: false,
            host: "localhost".to_string(),
            port: 7879,
            local: true,
            max_file_size: 5 * 1024 * 1024,
            use_token: false,
            token: "testtoken".to_string(),
            upload_mode: UploadMode::None,
            upload_whitelist: Vec::new(),
            upload_blacklist: Vec::new(),
        }
    }

    /// 通过TOML文件产生配置
    /// 
    /// ## 参数：
    /// - `filename`: TOML文件的路径
    pub fn from_toml(filename: &str) -> Self {
        // 打开文件
        let mut file = match File::open(filename) {
            Ok(f) => f,
            Err(e) => panic!("no such file {} exception:{}", filename, e)
        };
        // 读文件到str
        let mut str_val = String::new();
        match file.read_to_string(&mut str_val) {
            Ok(s) => s,
            Err(e) => panic!("Error Reading file: {}", e)
        };
        // 尝试读配置文件，若成功则返回，若失败则返回默认值
        let mut raw_config = match toml::from_str(&str_val) {
            Ok(t) => t,
            Err(_) => {
                println!("无法成功从配置文件构建配置对象，使用默认配置");
                Config::new()
            }
        };
        raw_config.max_file_size *= 1024 * 1024;
        raw_config
    }
}

impl Config {
    /// 获取 WWW root
    pub fn www_root(&self) -> &str {
        &self.www_root
    }

    /// 是否使用反向代理
    /// 
    /// 如果为true，则返回URL中不会带端口号；如果为false，则会带端口号。
    pub fn proxy(&self) -> bool {
        self.proxy
    }

    /// 是否使用SSL
    /// 
    /// 会决定返回的URL使用http还是https
    pub fn ssl(&self) -> bool {
        self.ssl
    }

    /// 获取主机名（用于返回URL）
    pub fn host(&self) -> &str {
        &self.host
    }

    /// 获取监听端口号
    pub fn port(&self) -> u16 {
        self.port
    }

    /// 检查服务是否工作在内网
    pub fn local(&self) -> bool {
        self.local
    }

    pub fn max_file_size(&self) -> usize {
        self.max_file_size
    }

    pub fn use_token(&self) -> bool {
        self.use_token
    }

    pub fn hashed_token(&self) -> String {
        get_str_sha256(&self.token)
    }

    pub fn upload_mode(&self) -> UploadMode {
        self.upload_mode.clone()
    }

    pub fn upload_whitelist(&self) -> Vec<String> {
        self.upload_whitelist.clone()
    }

    pub fn upload_blacklist(&self) -> Vec<String> {
        self.upload_blacklist.clone()
    }
}