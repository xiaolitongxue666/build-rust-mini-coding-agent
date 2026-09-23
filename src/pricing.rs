//! 第 16 课：[The token viewer](https://www.byoharness.dev/chapters/16-token-viewer.html)
//!
//! 每次响应的 usage 在发出那一刻计价，累进账本。会话跨过 12:00 / 18:00 时不能按「现在」重算整段。
//! 金额是人民币，前缀 `~`：费率会过期，静默的 ¥0 比未知更糟。
//!
//! 钟点是固定 UTC+8，不读本机时区。Windows、macOS、Linux、WSL 对同一个瞬间必须落在同一档。
//! 不读 `/etc/localtime`，也不读另一套 `$HOME`。
//!
//! 费率（2026-09-23，官方表是美元）：
//! <https://api-docs.deepseek.com/quick_start/pricing>
//! 高峰：周一到周五 09:00–12:00、14:00–18:00（北京时间，左闭右开）。
//! 对应 UTC 01:00–04:00 与 06:00–10:00。周末全天空闲。中国法定节假日全天空闲。
//! 空闲是高峰的一半。
//!
//! Flash 的人民币档（2026-09-10 公布）是美元 × 20/3：
//! 空闲命中 ¥0.02、未命中 ¥1、输出 ¥4；高峰翻倍。
//! 20/3 是官方核算汇率，不是实时外汇。Pro 用同一乘数乘官方美元表。
//!
//! 旧名 `deepseek-v4-flash`、`deepseek-v4-flash-vision-exp` 按 Flash 计。
//! 本仓库 `/model` 仍列出 `deepseek-chat`、`deepseek-reasoner`，官方说明由 V4.1-Flash 承接，也按 Flash。
//! `deepseek-v4-pro` 按 Pro。其它名字不计价，显示「未知模型」。
//!
//! 2026 放假与调休只认国务院办公厅国办发明电〔2025〕7号：
//! <https://www.gov.cn/zhengce/zhengceku/202511/content_7047091.htm>
//! 其它年份不套这张表。下一年的通知要另加。

use crate::api::Usage;

const CNY_PER_USD: f64 = 20.0 / 3.0;
const TOKENS_PER_MILLION: f64 = 1_000_000.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Beijing {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
}

pub fn beijing_now() -> Beijing {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    beijing_from_unix(secs)
}

pub fn beijing_from_unix(secs: i64) -> Beijing {
    let shifted = secs + 8 * 3600;
    let days = shifted.div_euclid(86_400);
    let second_of_day = shifted.rem_euclid(86_400) as u32;
    let (year, month, day) = civil_from_days(days);
    Beijing {
        year,
        month,
        day,
        hour: second_of_day / 3600,
    }
}

pub fn is_peak_at(when: Beijing) -> bool {
    if is_statutory_holiday(when.year, when.month, when.day) {
        return false;
    }
    let weekday = weekday_mon0(when.year, when.month, when.day);
    let weekend = weekday >= 5;
    if weekend && !is_makeup_workday(when.year, when.month, when.day) {
        return false;
    }
    (9..12).contains(&when.hour) || (14..18).contains(&when.hour)
}

struct Rates {
    hit_off: f64,
    miss_off: f64,
    out_off: f64,
}

fn rates_for(model: &str) -> Option<Rates> {
    match model {
        "deepseek-flash"
        | "deepseek-v4-flash"
        | "deepseek-v4-flash-vision-exp"
        | "deepseek-chat"
        | "deepseek-reasoner" => Some(Rates {
            hit_off: 0.02,
            miss_off: 1.0,
            out_off: 4.0,
        }),
        "deepseek-v4-pro" => Some(Rates {
            hit_off: 0.022 * CNY_PER_USD,
            miss_off: 0.66 * CNY_PER_USD,
            out_off: 1.98 * CNY_PER_USD,
        }),
        _ => None,
    }
}

/// `None` 表示这一个模型没有费率。调用方记一笔「未知」，不要当成 ¥0。
pub fn cost_cny(model: &str, peak: bool, usage: Usage) -> Option<f64> {
    let rates = rates_for(model)?;
    let scale = if peak { 2.0 } else { 1.0 };
    let yuan = scale
        * (rates.hit_off * usage.cache_read_tokens as f64
            + rates.miss_off * usage.input_tokens as f64
            + rates.out_off * usage.output_tokens as f64)
        / TOKENS_PER_MILLION;
    Some(yuan)
}

#[derive(Debug, Clone, Default)]
pub struct Ledger {
    pub peak: Usage,
    pub off_peak: Usage,
    yuan: f64,
    unpriced: bool,
}

impl Ledger {
    pub fn add(&mut self, model: &str, usage: Usage, when: Beijing) {
        let peak = is_peak_at(when);
        if peak {
            self.peak = self.peak.plus(usage);
        } else {
            self.off_peak = self.off_peak.plus(usage);
        }
        match cost_cny(model, peak, usage) {
            Some(yuan) => self.yuan += yuan,
            None => self.unpriced = true,
        }
    }

    pub fn report(&self) -> TokenReport {
        TokenReport {
            usage: self.peak.plus(self.off_peak),
            peak: self.peak,
            off_peak: self.off_peak,
            yuan: self.yuan,
            unpriced: self.unpriced,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TokenReport {
    pub usage: Usage,
    pub peak: Usage,
    pub off_peak: Usage,
    pub yuan: f64,
    pub unpriced: bool,
}

pub fn cost_text(report: &TokenReport) -> String {
    if report.unpriced && report.yuan == 0.0 {
        "未知模型".to_string()
    } else if report.unpriced {
        format!("~¥{:.4}（含未知模型）", report.yuan)
    } else {
        format!("~¥{:.4}", report.yuan)
    }
}

/// 还没打过模型时返回空串。空闲状态栏因此保持第 12 课的空白行。
pub fn usage_status(report: &TokenReport) -> String {
    if report.usage.is_zero() && !report.unpriced {
        return String::new();
    }
    let mut parts = vec![format!("{} in", commas(report.usage.input_tokens))];
    if report.usage.cache_read_tokens > 0 {
        parts.push(format!("{} hit", commas(report.usage.cache_read_tokens)));
    }
    parts.push(format!("{} out", commas(report.usage.output_tokens)));
    parts.push(cost_text(report));
    parts.join(" · ")
}

pub fn format_tokens(report: Option<&TokenReport>) -> String {
    let Some(report) = report else {
        return "this provider doesn't report token usage\n".to_string();
    };
    let mut out = String::from("session usage:\n");
    out.push_str(&format!(
        "  input (cache miss)  {}\n",
        commas(report.usage.input_tokens)
    ));
    out.push_str(&format!(
        "  output              {}\n",
        commas(report.usage.output_tokens)
    ));
    if report.usage.cache_read_tokens > 0 {
        out.push_str(&format!(
            "  cache hit           {}\n",
            commas(report.usage.cache_read_tokens)
        ));
    }
    if report.usage.cache_creation_tokens > 0 {
        out.push_str(&format!(
            "  cache write         {}\n",
            commas(report.usage.cache_creation_tokens)
        ));
    }
    out.push_str(&format!("  est. cost           {}\n", cost_text(report)));
    if let Some(line) = band_line("高峰", report.peak) {
        out.push_str(&line);
        out.push('\n');
    }
    if let Some(line) = band_line("空闲", report.off_peak) {
        out.push_str(&line);
        out.push('\n');
    }
    out
}

fn band_line(label: &str, usage: Usage) -> Option<String> {
    if usage.is_zero() {
        return None;
    }
    let mut parts = Vec::new();
    if usage.input_tokens > 0 {
        parts.push(format!("{} miss", commas(usage.input_tokens)));
    }
    if usage.cache_read_tokens > 0 {
        parts.push(format!("{} hit", commas(usage.cache_read_tokens)));
    }
    if usage.output_tokens > 0 {
        parts.push(format!("{} out", commas(usage.output_tokens)));
    }
    if usage.cache_creation_tokens > 0 {
        parts.push(format!(
            "{} cache write",
            commas(usage.cache_creation_tokens)
        ));
    }
    Some(format!("  {label}  {}", parts.join(" · ")))
}

pub fn commas(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (index, ch) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

fn is_statutory_holiday(year: i32, month: u32, day: u32) -> bool {
    if year != 2026 {
        return false;
    }
    matches!(
        (month, day),
        (1, 1..=3)
            | (2, 15..=23)
            | (4, 4..=6)
            | (5, 1..=5)
            | (6, 19..=21)
            | (9, 25..=27)
            | (10, 1..=7)
    )
}

fn is_makeup_workday(year: i32, month: u32, day: u32) -> bool {
    if year != 2026 {
        return false;
    }
    matches!(
        (month, day),
        (1, 4) | (2, 14) | (2, 28) | (5, 9) | (9, 20) | (10, 10)
    )
}

fn weekday_mon0(year: i32, month: u32, day: u32) -> u8 {
    let days = days_from_civil(year, month, day);
    // 1970-01-01 是周四。周一 = 0 时，周四 = 3。
    (days + 3).rem_euclid(7) as u8
}

fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let mut year = i64::from(year);
    if month <= 2 {
        year -= 1;
    }
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = (year - era * 400) as u64;
    let month_index = if month > 2 { month - 3 } else { month + 9 };
    let day_of_year = (153 * u64::from(month_index) + 2) / 5 + u64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era as i64 - 719_468
}

fn civil_from_days(mut z: i64) -> (i32, u32, u32) {
    z += 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = (z - era * 146_097) as u64;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36524 - day_of_era / 146_096) / 365;
    let year = year_of_era as i64 + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_part = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_part + 2) / 5 + 1;
    let month = if month_part < 10 {
        month_part + 3
    } else {
        month_part - 9
    };
    let year = year + if month <= 2 { 1 } else { 0 };
    (year as i32, month as u32, day as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(year: i32, month: u32, day: u32, hour: u32) -> Beijing {
        Beijing {
            year,
            month,
            day,
            hour,
        }
    }

    fn miss(tokens: u64) -> Usage {
        Usage {
            input_tokens: tokens,
            ..Usage::default()
        }
    }

    #[test]
    fn civil_epoch_and_known_weekdays() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(weekday_mon0(1970, 1, 1), 3);
        assert_eq!(weekday_mon0(2000, 1, 1), 5);
        assert_eq!(weekday_mon0(2026, 9, 23), 2);
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(days_from_civil(2026, 9, 23)), (2026, 9, 23));
    }

    #[test]
    fn beijing_midnight_is_utc_plus_8_not_local() {
        let unix = days_from_civil(2026, 9, 23) * 86_400 - 8 * 3600;
        assert_eq!(
            beijing_from_unix(unix),
            Beijing {
                year: 2026,
                month: 9,
                day: 23,
                hour: 0,
            }
        );
        assert_eq!(
            beijing_from_unix(0),
            Beijing {
                year: 1970,
                month: 1,
                day: 1,
                hour: 8,
            }
        );
    }

    #[test]
    fn peak_windows_holidays_and_makeup_days() {
        let monday = at(2026, 9, 21, 0);
        assert!(is_peak_at(Beijing { hour: 9, ..monday }));
        assert!(is_peak_at(Beijing { hour: 11, ..monday }));
        assert!(!is_peak_at(Beijing { hour: 12, ..monday }));
        assert!(!is_peak_at(Beijing { hour: 13, ..monday }));
        assert!(is_peak_at(Beijing { hour: 14, ..monday }));
        assert!(is_peak_at(Beijing { hour: 17, ..monday }));
        assert!(!is_peak_at(Beijing { hour: 18, ..monday }));
        assert!(!is_peak_at(at(2026, 9, 19, 10)));
        assert!(!is_peak_at(at(2026, 5, 4, 10)));
        assert!(!is_peak_at(at(2026, 9, 25, 10)));
        assert!(!is_peak_at(at(2026, 2, 16, 10)));
        assert!(is_peak_at(at(2026, 1, 4, 10)));
        assert!(is_peak_at(at(2026, 5, 9, 10)));
        assert!(is_peak_at(at(2027, 1, 4, 10)));
        assert!(!is_peak_at(at(2027, 1, 2, 10)));
    }

    #[test]
    fn flash_cny_is_the_published_rmb_table() {
        let off = at(2026, 9, 21, 20);
        let peak = at(2026, 9, 21, 10);
        assert!((cost_cny("deepseek-flash", false, miss(1_000_000)).unwrap() - 1.0).abs() < 1e-12);
        assert!((cost_cny("deepseek-flash", true, miss(1_000_000)).unwrap() - 2.0).abs() < 1e-12);
        let hit = Usage {
            cache_read_tokens: 1_000_000,
            ..Usage::default()
        };
        assert!((cost_cny("deepseek-flash", false, hit).unwrap() - 0.02).abs() < 1e-12);
        let out = Usage {
            output_tokens: 1_000_000,
            ..Usage::default()
        };
        assert!((cost_cny("deepseek-flash", false, out).unwrap() - 4.0).abs() < 1e-12);
        assert!(is_peak_at(peak));
        assert!(!is_peak_at(off));
        assert!(cost_cny("no-such-model", false, miss(1)).is_none());
        assert!((cost_cny("deepseek-chat", false, miss(1_000_000)).unwrap() - 1.0).abs() < 1e-12);
        assert!((cost_cny("deepseek-v4-pro", false, miss(1_000_000)).unwrap() - 4.4).abs() < 1e-9);
    }

    #[test]
    fn ledger_prices_each_call_and_hides_zero_cache() {
        let mut ledger = Ledger::default();
        ledger.add("deepseek-flash", miss(1_000_000), at(2026, 9, 21, 20));
        ledger.add(
            "deepseek-flash",
            Usage {
                cache_read_tokens: 1_000_000,
                ..Usage::default()
            },
            at(2026, 9, 21, 10),
        );
        let report = ledger.report();
        assert_eq!(report.off_peak.input_tokens, 1_000_000);
        assert_eq!(report.peak.cache_read_tokens, 1_000_000);
        assert!((report.yuan - 1.04).abs() < 1e-9);
        let detail = format_tokens(Some(&report));
        assert!(detail.contains("cache hit"));
        assert!(!detail.contains("cache write"));
        assert!(detail.contains("空闲"));
        assert!(detail.contains("高峰"));
        assert!(usage_status(&report).contains("~¥"));
        assert!(usage_status(&Ledger::default().report()).is_empty());

        let mut unknown = Ledger::default();
        unknown.add("no-such-model", miss(10), at(2026, 9, 21, 20));
        assert_eq!(cost_text(&unknown.report()), "未知模型");
        assert!(!cost_text(&unknown.report()).contains('¥'));

        ledger.add("no-such-model", miss(1), at(2026, 9, 21, 20));
        assert!(cost_text(&ledger.report()).contains("含未知模型"));
    }

    #[test]
    fn commas_group_thousands() {
        assert_eq!(commas(0), "0");
        assert_eq!(commas(999), "999");
        assert_eq!(commas(12_034), "12,034");
        assert_eq!(commas(1_000_000), "1,000,000");
    }
}
