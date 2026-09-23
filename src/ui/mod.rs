//! 第 04 课：[UI polish](https://www.byoharness.dev/chapters/04-ui-polish.html)
//! banner / spinner。第 08 课：一次性边框输入。不上第 12 课整屏 TUI。
//! 第 10 课：对应课上 `internal/ui/`。UI 可以认逻辑，逻辑不要反过来认 UI 以外的状态栏。

mod banner;
mod chat_input;
mod input;
mod spinner;

pub use banner::{banner_text, print_banner, term_width, BIG_BANNER_MIN_WIDTH, BIG_BANNER_WIDTH};
pub use chat_input::{
    append_history_to, load_history_from, render_box, ChatInputState, ChatKey, ChatOutcome,
};
pub use input::{ctrl_c_action, CtrlCAction, LineResult, PromptRead, ReplLine, SessionInput};
pub use spinner::{spin_until, stdout_is_tty, Wait};
