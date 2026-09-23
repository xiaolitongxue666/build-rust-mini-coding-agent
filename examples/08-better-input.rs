//! 第 08 课：[Better input](https://www.byoharness.dev/chapters/08-better-input.html)
//!
//! 本课小 demo：两步改善输入。实现在 crate 库一份，这里只接线。
//!
//! ```text
//! 步骤 1  bufio/stdin.lines  →  rustyline（第 04 课，80/20）
//! 步骤 2  rustyline          →  一次性边框输入（本课，MVU 热身）
//! ```
//!
//! 课上步骤 2 是 Bubble Tea。本仓库用 crossterm 一次性程序，不上第 12 课整屏 TUI。
//!
//! ```text
//! ╭─────────────────────────────────────────╮
//! │ ❯ your message                          │
//! ╰─────────────────────────────────────────╯
//!  enter: send · ↑↓: history · ctrl-d: exit
//! ```
//!
//! ↑ 开始翻历史前先把正在打的字放进 `buffer_text`；↓ 越过最新一条时还回去。
//! 历史写当前 `$HOME/.rustbyo_harness_history`（贴进去的密钥会留在磁盘上）。
//! confirm 走同一条读键路，不再 lock stdin。
//!
//! 非 TTY / `BYO_PLAIN_INPUT=1` 仍走 `stdin.lines()`。课上不管管道；本仓库要管。
//!
//! 启动：`bash scripts/run.sh 08`。总体：`bash scripts/run.sh`。不要写 API key。

use build_rust_mini_coding_agent::repl::run_repl;

fn main() {
    run_repl(true);
}

#[cfg(test)]
mod tests {
    use build_rust_mini_coding_agent::ui::{
        append_history_to, load_history_from, render_box, ChatInputState, ChatKey,
    };

    #[test]
    fn lesson_08_up_saves_draft() {
        let mut state = ChatInputState::new(40, vec!["older".into(), "newer".into()]);
        assert_eq!(
            state.apply(ChatKey::Char('d')),
            build_rust_mini_coding_agent::ui::ChatOutcome::Continue
        );
        assert_eq!(
            state.apply(ChatKey::Char('r')),
            build_rust_mini_coding_agent::ui::ChatOutcome::Continue
        );
        state.apply(ChatKey::Up);
        assert_eq!(state.value(), "newer");
        state.apply(ChatKey::Down);
        assert_eq!(state.value(), "dr");
    }

    #[test]
    fn lesson_08_box_view() {
        let view = render_box(&ChatInputState::new(40, Vec::new()));
        assert!(view.contains('╭'), "{view}");
        assert!(view.contains("enter: send"), "{view}");
    }

    #[test]
    fn lesson_08_history_skips_blank() {
        let path = std::env::temp_dir().join(format!(
            "byo-lesson08-{}.txt",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        append_history_to(&path, "  keep  ");
        append_history_to(&path, "   ");
        let lines = load_history_from(&path);
        let _ = std::fs::remove_file(&path);
        assert_eq!(lines, vec!["keep".to_string()]);
    }
}
