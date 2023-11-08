use std::{
    time::SystemTime,
    fs,
    path::Path,
};

/// # get_time
/// 
/// 用于获取系统时间。
/// 
/// 返回的是从 1970-01-01 00:00:00 UTC 起到现在的秒数。
pub fn get_time() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs()
}

/// # shorten
/// 
/// 用于缩短哈希值
pub fn shorten(input: &str) -> String {
    assert_eq!(input.len(), 64);
    let (left_half, right_half) = input.split_at(32);

    let left = u128::from_str_radix(left_half, 16).unwrap();
    let right = u128::from_str_radix(right_half, 16).unwrap();
    let result = left.wrapping_add(right);

    format!("{:x}", result)
}

/// # 格式化文件大小
/// 
/// ## 参数
/// - `size`: 以字节为单位的文件大小
/// 
/// ## 返回
/// - 格式化后的文件大小，原始大小的单位将被动态地调整到`B`、`KB`、`MB`、`GB`、`TB`等单位，并保留1位小数。
pub fn format_file_size(size: usize) -> String {
    if size == 0 {
        return "0 B".to_string();
    }
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut size = size as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < units.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{:.1} {}", size, units[unit_index])
}

pub fn calculate_total_size(directory_path: &str) -> u64 {
    let path = Path::new(directory_path);

    if !path.is_dir() {
        panic!("Path {} is not a directory!", directory_path);
    }

    let mut total_size = 0;

    for entry in fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        let metadata = entry.metadata().unwrap();

        if metadata.is_file() {
            total_size += metadata.len();
        } else if metadata.is_dir() {
            // 如果你想要递归计算子目录的文件大小，可以在这里调用递归函数
            // 例如：total_size += calculate_total_size(entry.path().to_str().unwrap())?;
        }
    }

    total_size
}