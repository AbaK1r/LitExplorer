use std::path::{Path, PathBuf};
use walkdir::{WalkDir, DirEntry};
use anyhow::{Context, Result};

/// 遍历日志目录，收集所有hparams.yaml文件路径
pub fn find_hparams_files(log_dir: &str, hparams_file: &str) -> Result<Vec<PathBuf>> {
    let path = Path::new(log_dir);

    // 检查目录是否存在
    if !path.exists() {
        anyhow::bail!("Log directory '{}' does not exist", log_dir);
    }

    if !path.is_dir() {
        anyhow::bail!("'{}' is not a directory", log_dir);
    }

    let mut hparams_files: Vec<PathBuf> = WalkDir::new(log_dir)
        .follow_links(true)
        .max_depth(2)
        .into_iter()
        .filter_map(Result::ok)             // 过滤掉错误条目
        .filter(|entry| is_hparams_file(entry, hparams_file)) // 保留符合条件的
        .map(|entry| entry.path().to_path_buf()) // 提取路径
        .collect();                          // 收集成 Vec

    // 按版本号排序（从目录名中提取）
    hparams_files.sort_by(|a, b| {
        let version_a = extract_version_number(a);
        let version_b = extract_version_number(b);
        version_a.cmp(&version_b)
    });

    Ok(hparams_files)
}

/// 从路径的父目录名中提取 "version_" 后的字符串部分（如 "version_42" → "42"）
fn extract_version_str_from_path(path: &Path) -> Option<String> {
    path.parent()
        .and_then(|p| p.file_name())
        .and_then(|name| {
            name.to_string_lossy()
                .strip_prefix("version_")
                .map(|s| s.to_string())
        })
}

/// 检查文件名是否存在且为 hparams_file 文件，且父目录名称为 "version_{number}"
fn is_hparams_file(entry: &DirEntry, hparams_file: &str) -> bool {
    entry.file_type().is_file()
        && entry.file_name() == hparams_file
        && extract_version_str_from_path(&entry.path())
        .and_then(|s| s.parse::<u32>().ok())
        .is_some()
}

/// 从文件路径中提取版本号
fn extract_version_number(path: &Path) -> u32 {
    extract_version_str_from_path(path)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

/// 从文件路径中提取版本号（带错误处理）
pub fn extract_version_number_safe(path: &Path) -> Result<u32> {
    let version_str = extract_version_str_from_path(path)
        .ok_or_else(|| anyhow::anyhow!("Failed to extract version number from path: {}", path.display()))?;

    version_str
        .parse()
        .with_context(|| format!("Failed to parse version number from: version_{}", version_str))
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    // 设置测试依赖
    #[test]
    fn test_extract_version_number() {
        // 测试有效的版本路径
        let path = Path::new("lightning_logs/version_123/hparams.yaml");
        assert_eq!(extract_version_number(path), 123);

        // 测试无效的路径格式
        let path = Path::new("lightning_logs/other_dir/hparams.yaml");
        assert_eq!(extract_version_number(path), 0);

        // 测试非版本目录
        let path = Path::new("lightning_logs/not_version/hparams.yaml");
        assert_eq!(extract_version_number(path), 0);
    }

    #[test]
    fn test_extract_version_number_safe() {
        // 测试有效的版本路径
        let path = Path::new("lightning_logs/version_456/hparams.yaml");
        assert_eq!(extract_version_number_safe(path).unwrap(), 456);

        // 测试无效的版本号
        let path = Path::new("lightning_logs/version_abc/hparams.yaml");
        assert!(extract_version_number_safe(path).is_err());

        // 测试非版本目录
        let path = Path::new("lightning_logs/other/hparams.yaml");
        assert!(extract_version_number_safe(path).is_err());
    }

    #[test]
    fn test_is_hparams_file() {
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();

        // 创建版本目录结构
        let version_dir = temp_path.join("version_1");
        fs::create_dir(&version_dir).unwrap();

        // 创建hparams文件
        let hparams_file = version_dir.join("hparams.yaml");
        fs::write(&hparams_file, "test").unwrap();

        // 创建非hparams文件
        let other_file = version_dir.join("other.yaml");
        fs::write(&other_file, "test").unwrap();

        // 创建非版本目录
        let other_dir = temp_path.join("other_dir");
        fs::create_dir(&other_dir).unwrap();
        let other_dir_file = other_dir.join("hparams.yaml");
        fs::write(&other_dir_file, "test").unwrap();

        let version_other_dir = temp_path.join("version_other");
        fs::create_dir(&version_other_dir).unwrap();
        let version_other_dir_file = version_other_dir.join("hparams.yaml");
        fs::write(&version_other_dir_file, "test").unwrap();

        // 使用WalkDir来获取DirEntry而不是直接构造
        let walker = WalkDir::new(temp_path);

        let mut entries = Vec::new();
        for entry in walker.into_iter().filter_map(|e| e.ok()) {
            entries.push(entry);
        }
        dbg!(&entries);

        // 查找对应的文件条目
        let hparams_entry = entries.iter()
            .find(|e| e.path() == hparams_file)
            .unwrap();

        let other_entry = entries.iter()
            .find(|e| e.path() == other_file)
            .unwrap();

        let other_dir_entry = entries.iter()
            .find(|e| e.path() == other_dir_file)
            .unwrap();

        let version_other_dir_entry = entries.iter()
            .find(|e| e.path() == version_other_dir_file)
            .unwrap();
        // 测试正确的hparams文件
        assert!(is_hparams_file(hparams_entry, "hparams.yaml"));

        // 测试错误的文件名
        assert!(!is_hparams_file(other_entry, "hparams.yaml"));

        // 测试非版本目录中的文件
        assert!(!is_hparams_file(other_dir_entry, "hparams.yaml"));

        // 测试非版本目录中的文件
        assert!(!is_hparams_file(version_other_dir_entry, "hparams.yaml"));
    }

    #[test]
    fn test_find_hparams_files() {
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();

        // 创建测试目录结构
        let logs_dir = temp_path.join("lightning_logs");
        fs::create_dir(&logs_dir).unwrap();

        // 创建多个版本目录
        for i in &[0, 1, 5, 10] {
            let version_dir = logs_dir.join(format!("version_{}", i));
            fs::create_dir(&version_dir).unwrap();

            let hparams_file = version_dir.join("hparams.yaml");
            fs::write(&hparams_file, format!("version: {}", i)).unwrap();

            let code_file = version_dir.join("code");
            fs::create_dir(&code_file).unwrap();
            let code_file = code_file.join("code.py");
            fs::write(&code_file, "code").unwrap();
        }

        // 创建一些干扰文件和目录
        let other_dir = logs_dir.join("other_dir");
        fs::create_dir(&other_dir).unwrap();
        let other_file = other_dir.join("hparams.yaml");
        fs::write(&other_file, "should not be found").unwrap();

        let config_dir = logs_dir.join("version_config");
        fs::create_dir(&config_dir).unwrap();
        let config_file = config_dir.join("hparams.yaml");
        fs::write(&config_file, "should not be found").unwrap();

        let walker = WalkDir::new(temp_path);
        let mut entries = Vec::new();
        for entry in walker.into_iter().filter_map(|e| e.ok()) {
            entries.push(entry);
        }
        dbg!(&entries);

        // 测试查找hparams文件
        let result = find_hparams_files(logs_dir.to_str().unwrap(), "hparams.yaml").unwrap();
        dbg!(&result);

        // 应该找到4个文件（版本0,1,5,10），并且按版本号排序
        assert_eq!(result.len(), 4);

        // 检查排序顺序
        let versions: Vec<u32> = result.iter()
            .map(|path| extract_version_number(path))
            .collect();

        assert_eq!(versions, vec![0, 1, 5, 10]);

        // 检查文件路径正确
        for (i, path) in result.iter().enumerate() {
            assert!(path.ends_with("hparams.yaml"));
            assert!(path.to_string_lossy().contains(&format!("version_{}", versions[i])));
        }
    }

    #[test]
    fn test_find_hparams_files_nonexistent_dir() {
        // 测试不存在的目录
        let result = find_hparams_files("/nonexistent/directory", "hparams.yaml");
        assert!(result.is_err());
    }

    #[test]
    fn test_find_hparams_files_file_instead_of_dir() {
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();

        // 创建一个文件而不是目录
        let file_path = temp_path.join("file.txt");
        fs::write(&file_path, "test").unwrap();

        // 测试文件而不是目录的情况
        let result = find_hparams_files(file_path.to_str().unwrap(), "hparams.yaml");
        assert!(result.is_err());
    }

    #[test]
    fn test_find_hparams_files_custom_filename() {
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();

        // 创建测试目录结构
        let logs_dir = temp_path.join("logs");
        fs::create_dir(&logs_dir).unwrap();

        // 创建版本目录和自定义文件名
        let version_dir = logs_dir.join("version_1");
        fs::create_dir(&version_dir).unwrap();

        let custom_file = version_dir.join("custom_params.yaml");
        fs::write(&custom_file, "test").unwrap();

        // 测试查找自定义文件名
        let result = find_hparams_files(logs_dir.to_str().unwrap(), "custom_params.yaml").unwrap();
        dbg!(&result);
        assert_eq!(result.len(), 1);
        assert!(result[0].ends_with("custom_params.yaml"));
    }

    #[test]
    fn test_find_hparams_files_empty_dir() {
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();

        // 创建空目录
        let empty_dir = temp_path.join("empty_logs");
        fs::create_dir(&empty_dir).unwrap();

        // 测试空目录
        let result = find_hparams_files(empty_dir.to_str().unwrap(), "hparams.yaml").unwrap();
        assert_eq!(result.len(), 0);
    }
}