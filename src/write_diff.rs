//! 第 18 课：[Diff approval for writes](https://www.byoharness.dev/chapters/18-diff-approval.html)
//!
//! 模型发来的是 `write_file` 的 path 和全文。磁盘还没动。这里读当前文件，算出统一 diff，
//! 交给审批。模型看不见这份 diff。拒绝时仍然只回 `user denied this tool call`。
//!
//! 只认本地工具名 `write_file`。MCP 的 `filesystem_write_file` 参数形状不一定是 path + content，
//! 继续走普通 `approve?`。
//!
//! diff 是按下 y/n 那一刻的快照。别的终端若同时改了文件，批准后写下去的是模型给出的全文。
//! 按文本行比较，不认二进制。

use std::fs;
use std::path::Path;

const CONTEXT: usize = 3;

/// 非 `write_file`、或参数解不开时，detail 为空，提示保持 `approve?`。
pub fn write_approval(name: &str, raw_input: &str) -> (String, String) {
    if name != "write_file" {
        return ("approve?".to_string(), String::new());
    }
    let Some((path, content)) = parse_write(raw_input) else {
        return ("approve?".to_string(), String::new());
    };
    let detail = build_write_diff(&path, &content);
    (format!("approve write to {path}?"), detail)
}

pub fn build_write_diff(path: &str, content: &str) -> String {
    match fs::read(Path::new(path)) {
        Ok(existing) => {
            let current = String::from_utf8_lossy(&existing);
            if current.as_ref() == content {
                "(no changes)\n".to_string()
            } else {
                unified(path, current.as_ref(), content)
            }
        }
        Err(_) => new_file(path, content),
    }
}

fn parse_write(raw_input: &str) -> Option<(String, String)> {
    let value: serde_json::Value = serde_json::from_str(raw_input).ok()?;
    let path = value.get("path")?.as_str()?.to_string();
    let content = value.get("content")?.as_str()?.to_string();
    if path.is_empty() {
        return None;
    }
    Some((path, content))
}

fn new_file(path: &str, content: &str) -> String {
    let lines = split_lines(content);
    let count = lines.len();
    let mut out = format!("--- /dev/null\n+++ {path} (new file)\n@@ -0,0 +1,{count} @@\n");
    for line in lines {
        out.push('+');
        out.push_str(show(&line));
        out.push('\n');
    }
    out
}

fn unified(path: &str, current: &str, proposed: &str) -> String {
    let old = split_lines(current);
    let new = split_lines(proposed);
    let records = edit_script(&old, &new);
    let mut out = format!("--- {path} (current)\n+++ {path} (proposed)\n");
    out.push_str(&hunks(&records));
    out
}

#[derive(Clone)]
struct Rec {
    kind: char,
    text: String,
    a_line: Option<usize>,
    b_line: Option<usize>,
}

fn edit_script(old: &[String], new: &[String]) -> Vec<Rec> {
    let n = old.len();
    let m = new.len();
    let mut lcs = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            if old[i] == new[j] {
                lcs[i][j] = lcs[i + 1][j + 1] + 1;
            } else {
                lcs[i][j] = lcs[i + 1][j].max(lcs[i][j + 1]);
            }
        }
    }
    let mut records = Vec::new();
    let mut i = 0;
    let mut j = 0;
    let mut a_no = 1usize;
    let mut b_no = 1usize;
    while i < n && j < m {
        if old[i] == new[j] {
            records.push(Rec {
                kind: ' ',
                text: old[i].clone(),
                a_line: Some(a_no),
                b_line: Some(b_no),
            });
            i += 1;
            j += 1;
            a_no += 1;
            b_no += 1;
        } else if lcs[i + 1][j] >= lcs[i][j + 1] {
            records.push(Rec {
                kind: '-',
                text: old[i].clone(),
                a_line: Some(a_no),
                b_line: None,
            });
            i += 1;
            a_no += 1;
        } else {
            records.push(Rec {
                kind: '+',
                text: new[j].clone(),
                a_line: None,
                b_line: Some(b_no),
            });
            j += 1;
            b_no += 1;
        }
    }
    while i < n {
        records.push(Rec {
            kind: '-',
            text: old[i].clone(),
            a_line: Some(a_no),
            b_line: None,
        });
        i += 1;
        a_no += 1;
    }
    while j < m {
        records.push(Rec {
            kind: '+',
            text: new[j].clone(),
            a_line: None,
            b_line: Some(b_no),
        });
        j += 1;
        b_no += 1;
    }
    records
}

fn hunks(records: &[Rec]) -> String {
    let changes: Vec<usize> = records
        .iter()
        .enumerate()
        .filter(|(_, rec)| rec.kind != ' ')
        .map(|(index, _)| index)
        .collect();
    if changes.is_empty() {
        return String::new();
    }
    let mut groups = Vec::new();
    let mut group_start = changes[0];
    let mut group_end = changes[0];
    for index in changes.into_iter().skip(1) {
        if index <= group_end + CONTEXT * 2 + 1 {
            group_end = index;
        } else {
            groups.push((group_start, group_end));
            group_start = index;
            group_end = index;
        }
    }
    groups.push((group_start, group_end));

    let mut out = String::new();
    for (change_start, change_end) in groups {
        let start = change_start.saturating_sub(CONTEXT);
        let end = (change_end + 1 + CONTEXT).min(records.len());
        let slice = &records[start..end];
        let a_count = slice.iter().filter(|rec| rec.kind != '+').count();
        let b_count = slice.iter().filter(|rec| rec.kind != '-').count();
        let a_start = if a_count == 0 {
            0
        } else {
            slice.iter().find_map(|rec| rec.a_line).unwrap_or(1)
        };
        let b_start = if b_count == 0 {
            0
        } else {
            slice.iter().find_map(|rec| rec.b_line).unwrap_or(1)
        };
        out.push_str(&format!(
            "@@ -{a_start},{a_count} +{b_start},{b_count} @@\n"
        ));
        for rec in slice {
            out.push(rec.kind);
            out.push_str(show(&rec.text));
            out.push('\n');
        }
    }
    out
}

fn split_lines(text: &str) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    let mut lines = Vec::new();
    let mut start = 0;
    for (index, byte) in text.as_bytes().iter().enumerate() {
        if *byte == b'\n' {
            lines.push(text[start..=index].to_string());
            start = index + 1;
        }
    }
    if start < text.len() {
        lines.push(text[start..].to_string());
    }
    lines
}

fn show(line: &str) -> &str {
    line.strip_suffix('\n').unwrap_or(line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_is_all_additions() {
        let path = std::env::temp_dir().join("byo-lesson18-missing.txt");
        let _ = fs::remove_file(&path);
        let text = build_write_diff(path.to_str().unwrap(), "hello\n");
        assert!(text.contains("--- /dev/null"), "{text}");
        assert!(text.contains("(new file)"), "{text}");
        assert!(text.contains("+hello"), "{text}");
        assert!(!text.contains("-hello"), "{text}");
    }

    #[test]
    fn identical_file_says_no_changes() {
        let path = std::env::temp_dir().join("byo-lesson18-same.txt");
        fs::write(&path, "same\n").unwrap();
        let text = build_write_diff(path.to_str().unwrap(), "same\n");
        let _ = fs::remove_file(&path);
        assert_eq!(text, "(no changes)\n");
    }

    #[test]
    fn changed_line_keeps_three_lines_of_context() {
        let path = std::env::temp_dir().join("byo-lesson18-edit.txt");
        let current = "alpha\nbeta\ngamma\ndelta\nepsilon\nzeta\neta\ntheta\niota\nkappa\n";
        fs::write(&path, current).unwrap();
        let proposed = current.replace("kappa", "KAPPA");
        let text = build_write_diff(path.to_str().unwrap(), &proposed);
        let _ = fs::remove_file(&path);
        assert!(text.contains("(current)"), "{text}");
        assert!(text.contains("(proposed)"), "{text}");
        assert!(text.contains("-kappa"), "{text}");
        assert!(text.contains("+KAPPA"), "{text}");
        assert!(text.contains(" theta"), "{text}");
        assert!(!text.contains(" alpha"), "{text}");
    }

    #[test]
    fn other_tool_names_skip_the_diff() {
        let (prompt, detail) =
            write_approval("filesystem_write_file", r#"{"path":"a","content":"b"}"#);
        assert_eq!(prompt, "approve?");
        assert!(detail.is_empty());
        let (prompt, detail) = write_approval("bash", "ls");
        assert_eq!(prompt, "approve?");
        assert!(detail.is_empty());
        let (prompt, detail) = write_approval("write_file", "not-json");
        assert_eq!(prompt, "approve?");
        assert!(detail.is_empty());
    }
}
