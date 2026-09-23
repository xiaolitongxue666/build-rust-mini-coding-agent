//! 第 14 课：stdio 夹具走一遍登记、前缀、坏 schema、调用、关闭。
//! 不读 API key，不下载 MCP server。

use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use build_rust_mini_coding_agent::mcp;
use build_rust_mini_coding_agent::tools::Registry;

fn fixture_bin() -> std::path::PathBuf {
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_mcp_stdio_fixture") {
        return std::path::PathBuf::from(path);
    }
    // 测试可执行文件在 target/<profile>/deps/。夹具 bin 在上一层。
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|deps| deps.parent())
        .expect("profile dir");
    let mut path = profile_dir.join("mcp-stdio-fixture");
    if cfg!(windows) {
        path.set_extension("exe");
    }
    assert!(path.is_file(), "missing fixture bin {}", path.display());
    path
}

#[test]
fn stdio_fixture_registers_and_calls() {
    let bin = fixture_bin();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("byo-mcp-fixture-{nanos}.json"));
    let config = serde_json::json!({
        "servers": [
            {
                "name": "missing",
                "transport": "stdio",
                "command": "byo-mcp-does-not-exist-xyz",
                "args": []
            },
            {
                "name": "fixture",
                "transport": "stdio",
                "command": bin,
                "args": []
            }
        ]
    });
    fs::write(&path, config.to_string()).expect("write config");

    let mut registry = Registry::new();
    let clients = mcp::setup_at(&path, &mut registry);
    let _ = fs::remove_file(&path);

    assert_eq!(clients.len(), 1, "bad server is skipped, fixture stays");
    let names: Vec<String> = registry
        .definitions()
        .into_iter()
        .map(|def| def.name)
        .collect();
    assert_eq!(
        names,
        vec![
            "fixture_blob".to_string(),
            "fixture_boom".to_string(),
            "fixture_current_time".to_string(),
        ]
    );
    assert!(!names.iter().any(|name| name.contains("broken")));

    let (text, is_err) = registry.execute("fixture_current_time", r#"{"tz":"UTC"}"#);
    assert!(!is_err, "{text}");
    assert_eq!(text, "ok:UTC");

    let (text, is_err) = registry.execute("fixture_blob", "{}");
    assert!(!is_err, "{text}");
    assert!(text.contains("[non-text content block: image]"), "{text}");

    let (text, is_err) = registry.execute("fixture_boom", "{}");
    assert!(is_err, "{text}");
    assert!(text.contains("nope"), "{text}");

    let (text, is_err) = registry.execute("fixture_current_time", "not-json");
    assert!(is_err, "{text}");
    assert!(text.contains("invalid tool input"), "{text}");

    mcp::close_all(&clients);
    let (text, is_err) = registry.execute("fixture_current_time", r#"{"tz":"UTC"}"#);
    assert!(is_err, "{text}");
    assert!(text.contains("closed"), "{text}");
}
