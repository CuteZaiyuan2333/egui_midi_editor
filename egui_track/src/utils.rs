//! 工具函数模块
//!
//! 包含通用的工具函数，如时间格式化等。

/// 将时间（秒）格式化为 "MM:SS.mmm" 格式
///
/// # 参数
///
/// * `time_seconds` - 时间（秒）
///
/// # 返回
///
/// 格式化后的时间字符串，格式为 "MM:SS.mmm"
///
/// # 示例
///
/// ```
/// use egui_track::utils::format_time;
///
/// let formatted = format_time(125.5);
/// assert_eq!(formatted, "02:05.500");
/// ```
pub fn format_time(time_seconds: f64) -> String {
    let minutes = (time_seconds / 60.0) as u32;
    let seconds = (time_seconds % 60.0) as u32;
    let milliseconds = ((time_seconds % 1.0) * 1000.0) as u32;
    format!("{:02}:{:02}.{:03}", minutes, seconds, milliseconds)
}

