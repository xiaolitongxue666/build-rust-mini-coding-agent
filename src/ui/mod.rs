//! 第 04 课：[UI polish](https://www.byoharness.dev/chapters/04-ui-polish.html)
//! banner / spinner。第 08 课：一次性边框输入。第 12 课：整屏程序。
//! 第 10 课：对应课上 `internal/ui/`。
//! 第 12 课：UI 读 `subagent::Active()`。循环不再 import 本模块。

mod banner;
mod chat_input;
mod input;
mod program;
mod spinner;
mod stdout_pipe;

pub use crate::gate::{LineResult, PromptRead};
pub use banner::{banner_text, print_banner, term_width, BIG_BANNER_MIN_WIDTH, BIG_BANNER_WIDTH};
pub use chat_input::{
    append_history_to, load_history_from, render_box, ChatInputState, ChatKey, ChatOutcome,
};
pub use input::{ctrl_c_action, CtrlCAction, ReplLine, SessionInput};
pub use program::{run_tui, Harness, HarnessCmd, HarnessEvent, ModelState};
pub use spinner::{spin_until, stdout_is_tty, Wait};

/// 第 11 课：委托头尾用暗色，和根 agent 的 `[tool]` 分开。
pub fn dimmed(text: &str) -> String {
    format!("\x1b[2m{text}\x1b[0m")
}
