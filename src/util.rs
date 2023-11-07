use std::time::SystemTime;

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
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut size = size as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < units.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{:.1} {}", size, units[unit_index])
}