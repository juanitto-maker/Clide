// ============================================
// scheduler.rs - Scheduled Tasks / Cron System
// ============================================
//
// Runs as a background tokio task alongside the bot polling loop.
// Checks every 60 seconds if any configured task is due, executes it,
// logs the result to the database, and sends a notification to the
// active chat platform.

use chrono::{Datelike, Timelike, Utc};
use log::{error, info, warn};
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use crate::config::{Config, ScheduledTask};
use crate::database::Database;
use crate::executor::Executor;
use crate::skills::SkillManager;

// ── Notification channel ─────────────────────────────────────────────────────

/// Describes how to send notifications about completed scheduled tasks.
#[derive(Clone)]
pub enum NotifyChannel {
    Telegram {
        client: crate::telegram::TelegramClient,
        /// Shared chat_id — updated dynamically when the first user message
        /// arrives so the scheduler can send notifications to the right chat.
        /// A value of 0 means "not yet known"; notifications are silently
        /// skipped until a real chat_id is set.
        chat_id: Arc<AtomicI64>,
        thread_id: Option<i64>,
    },
    Matrix {
        /// Matrix client wrapped in Arc<Mutex> because it requires `&mut self`
        /// for send_message (it increments a transaction counter).
        client: Arc<tokio::sync::Mutex<crate::matrix::MatrixClient>>,
    },
}

impl NotifyChannel {
    async fn send(&self, message: &str) {
        match self {
            NotifyChannel::Telegram {
                client,
                chat_id,
                thread_id,
            } => {
                let cid = chat_id.load(Ordering::Relaxed);
                if cid == 0 {
                    warn!("Scheduler: no chat_id known yet, skipping notification");
                    return;
                }
                if let Err(e) = client.send_message(cid, *thread_id, message).await {
                    warn!("Scheduler: failed to send Telegram notification: {}", e);
                }
            }
            NotifyChannel::Matrix { client } => {
                let mut c = client.lock().await;
                if let Err(e) = c.send_message(message).await {
                    warn!("Scheduler: failed to send Matrix notification: {}", e);
                }
            }
        }
    }
}

// ── Cron expression parser ───────────────────────────────────────────────────

/// A parsed cron expression with five fields:
/// minute, hour, day-of-month, month, day-of-week.
#[derive(Debug, Clone)]
struct CronExpr {
    minute: CronField,
    hour: CronField,
    dom: CronField,
    month: CronField,
    dow: CronField,
}

/// A single cron field that can match values.
#[derive(Debug, Clone)]
enum CronField {
    /// `*` — matches everything.
    Any,
    /// `*/N` — matches when value % N == 0.
    Step(u32),
    /// Explicit set of values (supports `1,15,30` and single numbers).
    Values(Vec<u32>),
}

impl CronField {
    fn matches(&self, value: u32) -> bool {
        match self {
            CronField::Any => true,
            CronField::Step(step) => *step > 0 && value % step == 0,
            CronField::Values(vals) => vals.contains(&value),
        }
    }
}

/// Parse a single cron field token (e.g. `*`, `*/5`, `1,15,30`, `3`).
fn parse_cron_field(token: &str) -> Result<CronField, String> {
    let token = token.trim();
    if token == "*" {
        return Ok(CronField::Any);
    }
    if let Some(step_str) = token.strip_prefix("*/") {
        let step: u32 = step_str
            .parse()
            .map_err(|_| format!("invalid step value in '{}'", token))?;
        if step == 0 {
            return Err("step value cannot be 0".to_string());
        }
        return Ok(CronField::Step(step));
    }
    // Comma-separated values or a single number
    let mut vals = Vec::new();
    for part in token.split(',') {
        let v: u32 = part
            .trim()
            .parse()
            .map_err(|_| format!("invalid number '{}' in cron field '{}'", part.trim(), token))?;
        vals.push(v);
    }
    Ok(CronField::Values(vals))
}

/// Parse a full 5-field cron expression string.
fn parse_cron(expr: &str) -> Result<CronExpr, String> {
    let fields: Vec<&str> = expr.split_whitespace().collect();
    if fields.len() != 5 {
        return Err(format!(
            "cron expression must have 5 fields (got {}): '{}'",
            fields.len(),
            expr
        ));
    }
    Ok(CronExpr {
        minute: parse_cron_field(fields[0])?,
        hour: parse_cron_field(fields[1])?,
        dom: parse_cron_field(fields[2])?,
        month: parse_cron_field(fields[3])?,
        dow: parse_cron_field(fields[4])?,
    })
}

impl CronExpr {
    /// Check whether the given UTC timestamp matches this cron expression.
    fn matches_timestamp(&self, ts: i64) -> bool {
        let dt = match chrono::DateTime::from_timestamp(ts, 0) {
            Some(dt) => dt,
            None => return false,
        };
        let minute = dt.minute();
        let hour = dt.hour();
        let dom = dt.day();
        let month = dt.month();
        // chrono: Monday=0 .. Sunday=6 via weekday().num_days_from_monday()
        // cron convention: Sunday=0, Monday=1 .. Saturday=6
        let dow = dt.weekday().num_days_from_sunday();

        self.minute.matches(minute)
            && self.hour.matches(hour)
            && self.dom.matches(dom)
            && self.month.matches(month)
            && self.dow.matches(dow)
        }
}

// ── Database helpers ─────────────────────────────────────────────────────────

fn open_db() -> Option<Database> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let db_path = format!("{}/.clide/memory.db", home);
    match Database::new(&db_path) {
        Ok(db) => Some(db),
        Err(e) => {
            warn!("Scheduler: could not open database: {}", e);
            None
        }
    }
}

// ── Task execution ───────────────────────────────────────────────────────────

/// Determine the task type string for logging.
fn task_type(task: &ScheduledTask) -> &'static str {
    if task.task.is_some() {
        "task"
    } else if task.skill.is_some() {
        "skill"
    } else if task.command.is_some() {
        "command"
    } else {
        "unknown"
    }
}

/// Execute a single scheduled task and return `(success, output_or_error)`.
async fn execute_task(
    task: &ScheduledTask,
    config: &Config,
) -> (bool, String) {
    if let Some(ref cmd) = task.command {
        // Raw shell command
        let executor = Executor::new(config.clone());
        match executor.execute(cmd).await {
            Ok(result) => {
                let output = result.output();
                (result.success(), output)
            }
            Err(e) => (false, format!("Execution error: {}", e)),
        }
    } else if let Some(ref skill_name) = task.skill {
        // Skill execution
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let skills_path = format!("{}/.clide/skills", home);
        match SkillManager::new(&skills_path) {
            Ok(sm) => {
                let executor = Executor::new(config.clone());
                match sm.execute_skill(skill_name, &task.params, &executor).await {
                    Ok(result) => {
                        let output: Vec<String> = result
                            .results
                            .iter()
                            .map(|r| r.output())
                            .collect();
                        (result.success, output.join("\n---\n"))
                    }
                    Err(e) => (false, format!("Skill error: {}", e)),
                }
            }
            Err(e) => (false, format!("Could not load skills: {}", e)),
        }
    } else if let Some(ref task_prompt) = task.task {
        // Natural language task — create a temporary Agent
        let mut agent = crate::agent::Agent::new(config);
        match agent.run(task_prompt, "scheduler", None, None).await {
            Ok(response) => (true, response),
            Err(e) => (false, format!("Agent error: {}", e)),
        }
    } else {
        (false, "No task, skill, or command defined".to_string())
    }
}

// ── Scheduler loop ───────────────────────────────────────────────────────────

/// Spawn the scheduler as a background tokio task.
///
/// The scheduler wakes every 60 seconds, checks which tasks are due
/// (comparing the current minute against the cron expression and the
/// last-run timestamp), executes them, logs results, and sends a
/// notification to the chat.
pub fn spawn(config: Config, notify: NotifyChannel) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        info!("Scheduler started with {} task(s)", config.scheduled_tasks.len());

        // Pre-parse cron expressions and warn about invalid ones.
        let mut parsed: Vec<(ScheduledTask, CronExpr)> = Vec::new();
        for task in &config.scheduled_tasks {
            if !task.enabled {
                info!("Scheduler: task '{}' is disabled, skipping", task.name);
                continue;
            }
            match parse_cron(&task.schedule) {
                Ok(cron) => {
                    info!(
                        "Scheduler: registered task '{}' with schedule '{}'",
                        task.name, task.schedule
                    );
                    parsed.push((task.clone(), cron));
                }
                Err(e) => {
                    error!(
                        "Scheduler: invalid cron expression '{}' for task '{}': {}",
                        task.schedule, task.name, e
                    );
                }
            }
        }

        if parsed.is_empty() {
            info!("Scheduler: no enabled tasks with valid schedules, exiting");
            return;
        }

        // Track last-run timestamps in memory (seeded from DB).
        let mut last_runs: HashMap<String, i64> = HashMap::new();
        if let Some(db) = open_db() {
            for (task, _) in &parsed {
                if let Ok(Some(ts)) = db.get_last_run(&task.name) {
                    last_runs.insert(task.name.clone(), ts);
                }
            }
        }

        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;

            let now = Utc::now().timestamp();
            // Truncate to the start of the current minute for matching.
            let current_minute = now - (now % 60);

            for (task, cron) in &parsed {
                // Check if this task is due: the cron expression matches the
                // current minute AND we haven't already run in this minute.
                if !cron.matches_timestamp(current_minute) {
                    continue;
                }
                if let Some(&last) = last_runs.get(&task.name) {
                    if last >= current_minute {
                        continue; // already ran this minute
                    }
                }

                info!("Scheduler: executing task '{}'", task.name);
                last_runs.insert(task.name.clone(), current_minute);

                let started_at = Utc::now().timestamp();
                let (success, output) = execute_task(task, &config).await;
                let finished_at = Utc::now().timestamp();

                // Log to database
                if let Some(db) = open_db() {
                    if let Err(e) = db.log_scheduled_run(
                        &task.name,
                        task_type(task),
                        started_at,
                        Some(finished_at),
                        success,
                        Some(&output),
                        if success { None } else { Some(&output) },
                    ) {
                        warn!("Scheduler: failed to log run for '{}': {}", task.name, e);
                    }
                }

                // Send notification
                let status = if success { "completed" } else { "FAILED" };
                // Truncate output for notification to avoid flooding chat.
                let truncated_output = if output.len() > 500 {
                    format!("{}...", &output[..500])
                } else {
                    output
                };
                let notification = format!(
                    "[Scheduled] {} — {}\nType: {}\n{}",
                    task.name,
                    status,
                    task_type(task),
                    truncated_output,
                );
                notify.send(&notification).await;
            }
        }
    })
}

// ── /schedule command output ─────────────────────────────────────────────────

/// Build a human-readable summary of all scheduled tasks for the /schedule command.
pub fn build_schedule_message(config: &Config) -> String {
    if config.scheduled_tasks.is_empty() {
        return "No scheduled tasks configured.\n\
                Add tasks under `scheduled_tasks:` in ~/.clide/config.yaml"
            .to_string();
    }

    let db = open_db();
    let now = Utc::now();

    let mut lines = vec!["Scheduled Tasks\n".to_string()];

    for task in &config.scheduled_tasks {
        let enabled_icon = if task.enabled { "ON" } else { "OFF" };
        let ttype = task_type(task);

        // Last run info from DB
        let last_run_str = db
            .as_ref()
            .and_then(|d| d.get_last_run(&task.name).ok().flatten())
            .map(|ts| {
                match chrono::DateTime::from_timestamp(ts, 0) {
                    Some(dt) => {
                        let ago = now.signed_duration_since(dt);
                        if ago.num_hours() > 0 {
                            format!("{}h ago", ago.num_hours())
                        } else {
                            format!("{}m ago", ago.num_minutes())
                        }
                    }
                    None => "unknown".to_string(),
                }
            })
            .unwrap_or_else(|| "never".to_string());

        // Next run estimate (simple: find next matching minute within 24h)
        let next_run_str = if task.enabled {
            match parse_cron(&task.schedule) {
                Ok(cron) => {
                    let mut check = now.timestamp();
                    // Round up to next minute boundary
                    check = check - (check % 60) + 60;
                    let mut found = "within 24h".to_string();
                    for _ in 0..1440 {
                        // 1440 minutes = 24 hours
                        if cron.matches_timestamp(check) {
                            match chrono::DateTime::from_timestamp(check, 0) {
                                Some(dt) => {
                                    found = format!(
                                        "{:02}:{:02} UTC",
                                        dt.hour(),
                                        dt.minute()
                                    );
                                }
                                None => {}
                            }
                            break;
                        }
                        check += 60;
                    }
                    found
                }
                Err(_) => "invalid cron".to_string(),
            }
        } else {
            "disabled".to_string()
        };

        lines.push(format!(
            "  {} [{}] ({})\n    Schedule: {}\n    Last run: {} | Next: {}",
            task.name, enabled_icon, ttype, task.schedule, last_run_str, next_run_str,
        ));
    }

    // Stats from DB
    if let Some(ref db) = db {
        if let Ok(stats) = db.get_scheduled_stats() {
            if !stats.is_empty() {
                lines.push("\nRun Statistics:".to_string());
                for (name, _last, total, successes) in &stats {
                    lines.push(format!(
                        "  {} — {}/{} successful",
                        name, successes, total
                    ));
                }
            }
        }
    }

    lines.join("\n")
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cron_star() {
        let cron = parse_cron("* * * * *").unwrap();
        // Should match any timestamp
        let ts = Utc::now().timestamp();
        let minute_start = ts - (ts % 60);
        assert!(cron.matches_timestamp(minute_start));
    }

    #[test]
    fn test_parse_cron_specific() {
        // 30 2 * * * => minute=30, hour=2
        let cron = parse_cron("30 2 * * *").unwrap();
        // 2024-01-15 02:30:00 UTC
        let ts = chrono::NaiveDate::from_ymd_opt(2024, 1, 15)
            .unwrap()
            .and_hms_opt(2, 30, 0)
            .unwrap()
            .and_utc()
            .timestamp();
        assert!(cron.matches_timestamp(ts));

        // 2024-01-15 03:30:00 UTC should not match (hour=3 != 2)
        let ts2 = chrono::NaiveDate::from_ymd_opt(2024, 1, 15)
            .unwrap()
            .and_hms_opt(3, 30, 0)
            .unwrap()
            .and_utc()
            .timestamp();
        assert!(!cron.matches_timestamp(ts2));
    }

    #[test]
    fn test_parse_cron_step() {
        // */15 * * * * => every 15 minutes (0, 15, 30, 45)
        let cron = parse_cron("*/15 * * * *").unwrap();
        let ts_0 = chrono::NaiveDate::from_ymd_opt(2024, 1, 15)
            .unwrap()
            .and_hms_opt(10, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp();
        assert!(cron.matches_timestamp(ts_0));

        let ts_15 = chrono::NaiveDate::from_ymd_opt(2024, 1, 15)
            .unwrap()
            .and_hms_opt(10, 15, 0)
            .unwrap()
            .and_utc()
            .timestamp();
        assert!(cron.matches_timestamp(ts_15));

        let ts_7 = chrono::NaiveDate::from_ymd_opt(2024, 1, 15)
            .unwrap()
            .and_hms_opt(10, 7, 0)
            .unwrap()
            .and_utc()
            .timestamp();
        assert!(!cron.matches_timestamp(ts_7));
    }

    #[test]
    fn test_parse_cron_comma_values() {
        // 0,30 * * * * => minute 0 and 30
        let cron = parse_cron("0,30 * * * *").unwrap();
        let ts_0 = chrono::NaiveDate::from_ymd_opt(2024, 1, 15)
            .unwrap()
            .and_hms_opt(10, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp();
        assert!(cron.matches_timestamp(ts_0));

        let ts_30 = chrono::NaiveDate::from_ymd_opt(2024, 1, 15)
            .unwrap()
            .and_hms_opt(10, 30, 0)
            .unwrap()
            .and_utc()
            .timestamp();
        assert!(cron.matches_timestamp(ts_30));

        let ts_15 = chrono::NaiveDate::from_ymd_opt(2024, 1, 15)
            .unwrap()
            .and_hms_opt(10, 15, 0)
            .unwrap()
            .and_utc()
            .timestamp();
        assert!(!cron.matches_timestamp(ts_15));
    }

    #[test]
    fn test_parse_cron_invalid() {
        assert!(parse_cron("* * *").is_err()); // too few fields
        assert!(parse_cron("*/0 * * * *").is_err()); // step of 0
        assert!(parse_cron("abc * * * *").is_err()); // non-numeric
    }
}
