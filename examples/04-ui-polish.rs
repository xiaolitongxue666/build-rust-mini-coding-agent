//! 第 04 课：[UI polish](https://www.byoharness.dev/chapters/04-ui-polish.html)
//!
//! 本课小 demo：字标、窄终端回退、API 等待 spinner。实现在 crate 库里一份，
//! 这里只接线 `run_repl(true)`。字标是 ANSI Shadow 的 **RUSTBYO**，不是课上的 BETTATECH。
//!
//! ```text
//! [启动]
//!     │
//!     ▼
//! [term_width >= 69?]─no──▶ [单行 RUSTBYO]
//!     │
//!    yes
//!     │
//!     ▼
//! [大字标 + 暗灰副标题]
//!     │
//!     ▼
//! [> 读一行]  ← rustyline：↑↓ 历史，Ctrl+C 清空 / 两次退出
//!     │
//!     ▼
//! [spinner thinking...]  ← 只包住 llm.send
//!     │
//!     ├─ Esc ──▶ 停 spinner，丢掉本回合，回到 >
//!     │
//!     ▼
//! [打文本或 [tool]]
//! ```
//!
//! 非 TTY（`run > log.txt`）`GetSize` 失败当 0 列，走小字标；spinner 是空壳。
//! `Stop` 必须等线程清行，否则下一帧会盖住模型输出。
//! ANSI 只打在启动和 spinner，不准进 system prompt（会毁掉第 06 课 cache）。
//!
//! 启动：`bash scripts/run.sh 04`。总体：`bash scripts/run.sh`。不要写 API key。

use build_rust_mini_coding_agent::repl::run_repl;

fn main() {
    run_repl(true);
}

#[cfg(test)]
mod tests {
    use build_rust_mini_coding_agent::ui::{
        banner_text, ctrl_c_action, CtrlCAction, BIG_BANNER_MIN_WIDTH, BIG_BANNER_WIDTH,
    };

    #[test]
    fn lesson_04_wide_banner_is_block_art() {
        assert_eq!(BIG_BANNER_WIDTH, 66);
        let text = banner_text(BIG_BANNER_MIN_WIDTH);
        assert!(text.contains('█'), "{text}");
        assert!(text.contains("DeepSeek"), "{text}");
    }

    #[test]
    fn lesson_04_narrow_banner_is_wordmark() {
        let text = banner_text(40);
        assert!(text.contains("RUSTBYO"), "{text}");
        assert!(!text.contains('█'), "{text}");
    }

    #[test]
    fn lesson_04_ctrl_c_state_machine() {
        assert_eq!(ctrl_c_action(1), CtrlCAction::Clear);
        assert_eq!(ctrl_c_action(2), CtrlCAction::Quit);
    }
}
