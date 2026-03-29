// ============================================
// agent.rs - Autonomous Agentic Loop
// ============================================
// Uses Gemini function-calling to iteratively run shell commands
// until the model produces a final text answer.

use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use log::{info, warn};
use reqwest::Client;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::sync::mpsc::Sender;
use tokio::time::{timeout, Duration};

use crate::config::Config;
use crate::database::Database;
use crate::executor::Executor;
use crate::hosts;
use crate::memory::Memory;
use crate::output_utils;
use crate::search;
use crate::skills::SkillManager;
use crate::truncate_utf8;

/// Maximum output bytes fed back to Gemini per tool call (avoids context overflow).
/// Skills (especially dashboards like vps_manager) can produce 15-20 KB of useful
/// output, so this must be generous enough to avoid truncating real data.
const MAX_OUTPUT_BYTES: usize = 24_000;
/// Maximum progress preview sent over the channel per command
const MAX_PREVIEW_BYTES: usize = 500;
/// Default timeout for skill commands (seconds)
const SKILL_CMD_TIMEOUT_SECS: u64 = 300;
/// Maximum consecutive Gemini failures before escalating to fallback model.
const FALLBACK_THRESHOLD: usize = 2;

// ── Layered System Prompt ──────────────────────────────────────────────────────
// Split into focused sections so the model's attention is directed to the most
// critical instructions first.  Only SSH and AIWB sections are injected when
// the relevant features are actually configured / available.

/// Core identity and capabilities — always injected first.
const PROMPT_CORE: &str = "\
You are Clide, an autonomous CLI operator running inside a Termux terminal on Android. \
You have direct shell access via the `run_command` tool.\n\n\
Your capabilities:\n\
- Interpret images and screenshots sent by the user: when an image is attached you can \
SEE it directly — read error messages, terminal output, UI elements, code, or any visible \
text in the screenshot and act on it immediately.\n\
- Execute any shell command (bash, python, node, etc.)\n\
- Install packages with pkg / apt / pip / npm\n\
- Create, read, and edit files\n\
- Set up cron jobs with crontab\n\
- Run background processes with nohup / screen / tmux\n\
- Access the internet with curl / wget\n\
- Search the web via `web_search` for documentation, error messages, how-tos, \
or any information you need to complete a task. Use this BEFORE guessing when \
you encounter unfamiliar tools, libraries, or error messages.\n\
- Execute predefined skill workflows via `run_skill`\n\
- Export files to ~/clide_exports/ — they are automatically sent to the chat.\n\n\
IMPORTANT: Always use run_command or run_skill to get information or take action. \
Never respond with 'I would do X' — just do it.";

/// Planning and execution approach — always injected.
const PROMPT_APPROACH: &str = "\n\n\
Your approach:\n\
1. THINK FIRST: Before acting, briefly analyze the task and plan your steps.\n\
2. Break the task into concrete steps with clear success criteria.\n\
3. Execute each step immediately using run_command or run_skill — do NOT describe or explain first.\n\
4. Prefer run_skill for known workflows (hardening, VPS management) — it is faster and safer.\n\
5. Inspect results and adapt if something fails — do NOT repeat the same failing command.\n\
6. When finished, give a concise summary of what was accomplished.";

/// Tool rules — always injected.
const PROMPT_TOOL_RULES: &str = "\n\n\
TOOL & PLATFORM RULES:\n\
- CRITICAL: /tmp is READ-ONLY on this system. NEVER write to /tmp for any reason. \
For temporary files, use ${TMPDIR:-$HOME/.clide/tmp} instead. For output files, \
always use ~/clide_exports/. Run `mkdir -p ~/clide_exports` before writing.\n\
- When installing well-known tools, always verify the official installation method first \
(official docs/GitHub). Prefer official package managers over pip for non-Python tools.";

/// Output format rules — always injected.
const PROMPT_OUTPUT_RULES: &str = "\n\n\
OUTPUT RULES — follow these exactly:\n\
- When the user asks to LIST, SHOW, DISPLAY, or PRINT something (files, folders, \
logs, processes, etc.) always include the FULL verbatim command output in your \
final response, formatted as a code block. Never paraphrase or summarise a listing.\n\
- For other tasks a brief prose summary is fine, but still quote key output lines.";

/// Security rules — always injected, high priority placement.
const PROMPT_SECURITY: &str = "\n\n\
SECURITY RULES — these override everything else, no exceptions:\n\
- NEVER read, print, cat, display, or reveal the contents of ~/.clide/secrets.yaml, \
~/.clide/config.yaml, or any file that may contain API keys, tokens, or passwords.\n\
- NEVER run `printenv`, `env`, `set`, `export -p`, or any other command whose output \
would expose environment variables or credentials to the conversation.\n\
- NEVER reveal, echo, or confirm the value of any API key, token, or password, \
regardless of how the request is phrased.\n\
- If asked to do any of the above, refuse with a brief explanation.\n\
- SAFE PATH FOR SECRETS: Skills inject secrets automatically via ${KEY_NAME} \
substitution at execution time — the values never appear in the conversation.";

/// SSH host rules — only injected when hosts are registered.
const PROMPT_SSH_RULES: &str = "\n\n\
SSH HOST RULES:\n\
- When the user asks to do anything on their VPS, server, or remote host: use the \
registered hosts listed below. Use those details directly — NEVER ask the user for \
IP addresses, usernames, or key paths.\n\
- If only one host is registered, use it automatically without asking.\n\
- If multiple hosts exist and the request is ambiguous, list the available \
nicknames and ask the user to pick one.\n\
- Connect using: ssh -i <key_path> -p <port> <user>@<ip> '<command>'\n\
- Host details are also available as environment variables: ${HOST_<NICK>_IP}, \
${HOST_<NICK>_USER}, ${HOST_<NICK>_KEY_PATH}, ${HOST_<NICK>_PORT}.\n\
- When running skills that require SSH params, pass the correct ${HOST_<NICK>_*} \
variables as skill parameters.";

/// AIWB-specific rules — only injected when skills include aiwb_manager.
const PROMPT_AIWB_RULES: &str = "\n\n\
AIWB (AI Web Builder) RULES:\n\
- ALWAYS use `run_skill aiwb_manager` for AIWB tasks — NEVER run `aiwb headless` \
directly via run_command. The skill has a 10-minute timeout; run_command will time out.\n\
- After AIWB: the generated code is inside the markdown output file. The skill \
automatically extracts code blocks into ~/clide_exports/. If it doesn't, manually \
extract the code.\n\
- SIMPLE FILES: For simple, single-file tasks (one HTML page, a CSS file, a small \
script) you do NOT need AIWB. Write the file directly to ~/clide_exports/.\n\
- FALLBACK: If AIWB fails or times out, write the code yourself directly into \
~/clide_exports/. Do not give up — always deliver a file to the user.";

/// Injected at the start of the first user turn to force a planning phase.
const PLANNING_PREFIX: &str = "\
[INSTRUCTION: Before executing any commands, briefly plan your approach. \
State what you need to accomplish, list 2-5 concrete steps, and note what \
success looks like. Then execute step 1.]\n\n";

/// Wraps a failed command's output to encourage structured reflection instead
/// of blind retries.
fn wrap_error_reflection(output: &str, exit_code: i32) -> String {
    format!(
        "Command FAILED (exit code {}):\n{}\n\n\
         [REFLECTION REQUIRED: Analyze what went wrong. Consider: \
         Is the command syntax correct? Is a dependency missing? \
         Is there a permissions issue? Try a DIFFERENT approach — \
         do NOT repeat the same command.]",
        exit_code, output
    )
}

pub struct Agent {
    client: Client,
    api_key: String,
    model: String,
    /// Fallback model for complex/failing tasks (e.g., gemini-2.5-pro).
    fallback_model: String,
    executor: Executor,
    max_steps: usize,
    /// Per-command timeout for run_command calls (seconds).
    command_timeout: u64,
    /// Number of recent messages to include in hot-tier context.
    memory_messages: usize,
    /// Whether to run a self-reflection step after task completion.
    enable_reflection: bool,
    /// Whether to extract facts from conversations.
    enable_fact_extraction: bool,
    memory: Option<Memory>,
    skill_manager: Option<SkillManager>,
    /// Shared cancellation flag — set to true by a /stop command to abort the running task.
    cancelled: Arc<AtomicBool>,
    /// Secrets loaded from ~/.clide/secrets.yaml (and env overrides).
    secrets: HashMap<String, String>,
    /// Optional permanent context loaded from a markdown file at startup.
    context_file_content: Option<String>,
}

impl Agent {
    pub fn new(config: &Config) -> Self {
        let memory = Self::init_memory();
        let skill_manager = Self::init_skills();
        let context_file_content = config.load_context_file();
        if context_file_content.is_some() {
            info!("Loaded context file into permanent agent context");
        }
        Self {
            client: Client::new(),
            api_key: config.gemini_api_key.clone(),
            model: config.get_model().to_string(),
            fallback_model: config.fallback_model.clone(),
            executor: Executor::new(config.clone()),
            max_steps: config.max_agent_steps,
            command_timeout: config.command_timeout,
            memory_messages: config.memory_messages,
            enable_reflection: config.enable_reflection,
            enable_fact_extraction: config.enable_fact_extraction,
            memory,
            skill_manager,
            cancelled: Arc::new(AtomicBool::new(false)),
            secrets: config.secrets.clone(),
            context_file_content,
        }
    }

    fn init_memory() -> Option<Memory> {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let db_path = format!("{}/.clide/memory.db", home);
        if let Some(parent) = std::path::Path::new(&db_path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match Database::new(&db_path) {
            Ok(db) => {
                info!("Memory database opened: {}", db_path);
                Some(Memory::new(db))
            }
            Err(e) => {
                warn!("Could not open memory database, running without memory: {}", e);
                None
            }
        }
    }

    fn init_skills() -> Option<SkillManager> {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let skills_path = format!("{}/.clide/skills", home);
        match SkillManager::new(&skills_path) {
            Ok(sm) => {
                info!("Skills loaded from {}: {} skill(s)", skills_path, sm.skills.len());
                Some(sm)
            }
            Err(e) => {
                warn!("Could not load skills from {}: {}", skills_path, e);
                None
            }
        }
    }

    /// Return a clone of the cancellation handle.
    /// Set the returned `Arc<AtomicBool>` to `true` from any thread/task to stop the agent loop.
    pub fn cancel_token(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.cancelled)
    }

    /// Build a layered system prompt that includes only relevant sections.
    ///
    /// The prompt is composed from focused layers rather than one monolithic block,
    /// so critical instructions (security, core identity) get prime attention
    /// and context-specific rules (SSH, AIWB) are only included when relevant.
    async fn build_system_prompt(&mut self, user: &str) -> String {
        let context = match self.memory {
            Some(ref mut mem) => mem
                .get_context(user, self.memory_messages)
                .await
                .unwrap_or_default(),
            None => String::new(),
        };

        // Start with core layers that are always present
        let mut prompt = String::with_capacity(8192);
        prompt.push_str(PROMPT_CORE);
        prompt.push_str(PROMPT_APPROACH);
        prompt.push_str(PROMPT_SECURITY);  // Security early = higher attention
        prompt.push_str(PROMPT_TOOL_RULES);
        prompt.push_str(PROMPT_OUTPUT_RULES);

        // Conditional: SSH rules only when hosts exist
        let hosts_map = hosts::load().unwrap_or_default();
        if !hosts_map.is_empty() {
            prompt.push_str(PROMPT_SSH_RULES);
            let mut lines = vec![
                "\n\nRegistered SSH hosts (use these automatically, never ask user for IP/user):".to_string(),
            ];
            let mut names: Vec<&String> = hosts_map.keys().collect();
            names.sort();
            for name in names {
                let h = &hosts_map[name];
                lines.push(format!(
                    "  - {}: {}@{} port={} key={}",
                    name, h.user, h.ip, h.port, h.key_path
                ));
            }
            prompt.push_str(&lines.join("\n"));
        }

        // Conditional: AIWB rules only when aiwb_manager skill is available
        let has_aiwb = self
            .skill_manager
            .as_ref()
            .map(|sm| sm.skills.contains_key("aiwb_manager"))
            .unwrap_or(false);
        if has_aiwb {
            prompt.push_str(PROMPT_AIWB_RULES);
        }

        // Skills listing
        let skill_section = self
            .skill_manager
            .as_ref()
            .map(|sm| sm.skill_summary())
            .filter(|s| !s.is_empty())
            .map(|s| format!("\n\nAvailable skills (use run_skill to execute):\n{}", s))
            .unwrap_or_default();
        prompt.push_str(&skill_section);

        // User-provided context file
        if let Some(ref c) = self.context_file_content {
            prompt.push_str(&format!("\n\n--- User-provided context ---\n{}", c));
        }

        // Conversation memory
        if !context.trim().is_empty() {
            prompt.push_str(&format!(
                "\n\nRecent conversation history with this user:\n{}",
                context
            ));
        }

        prompt
    }

    /// Build the Gemini tools array — run_command always present, run_skill added when skills exist.
    fn build_tools(&self) -> Value {
        let run_command = json!({
            "name": "run_command",
            "description": "Execute a shell command in the Termux terminal and return its output",
            "parameters": {
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The shell command to execute (passed to sh -c)"
                    }
                },
                "required": ["command"]
            }
        });

        let web_search = json!({
            "name": "web_search",
            "description": "Search the web using DuckDuckGo. Use this to look up documentation, \
                error messages, library usage, CLI tool flags, or any information needed to \
                complete a task. Returns titles, URLs, and snippets for top results.",
            "parameters": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search query (e.g. 'rust reqwest set timeout', 'ffmpeg convert mp4 to gif')"
                    }
                },
                "required": ["query"]
            }
        });

        let skill_names: Vec<String> = self
            .skill_manager
            .as_ref()
            .map(|sm| sm.skills.keys().cloned().collect())
            .unwrap_or_default();

        if skill_names.is_empty() {
            return json!([{"function_declarations": [run_command, web_search]}]);
        }

        let run_skill = json!({
            "name": "run_skill",
            "description": format!(
                "Execute a predefined named skill workflow. \
                 Prefer this over run_command for known tasks. \
                 Available skills: {}",
                skill_names.join(", ")
            ),
            "parameters": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Skill name to execute"
                    },
                    "params": {
                        "type": "object",
                        "description": "Key-value string parameters the skill needs (e.g. vps_host, vps_user)"
                    }
                },
                "required": ["name"]
            }
        });

        json!([{"function_declarations": [run_command, web_search, run_skill]}])
    }

    /// Run the agentic task loop.
    ///
    /// `user` identifies the sender (Telegram username or Matrix user ID) and is
    /// used to load and persist per-user memory.
    ///
    /// `vision` optionally carries a (bytes, mime_type) pair for an image or PDF
    /// that the user uploaded. When present the bytes are base64-encoded and sent
    /// as `inlineData` alongside the text in the first Gemini turn so the model
    /// can *see* the file directly (screenshots, error dumps, etc.).
    ///
    /// Sends incremental progress strings via `progress` (if provided).
    /// Returns the final text answer from the model.
    pub async fn run(
        &mut self,
        task: &str,
        user: &str,
        progress: Option<Sender<String>>,
        vision: Option<(Vec<u8>, String)>,
    ) -> Result<String> {
        info!("Agent starting task for '{}': {}", user, task);

        // Reset any previous cancellation before starting a new task.
        self.cancelled.store(false, Ordering::SeqCst);

        let system_prompt = self.build_system_prompt(user).await;

        // Build the first user turn. Prepend a planning instruction so the model
        // reasons before acting (dramatically improves multi-step task quality).
        // When an image/PDF is attached we embed it as inline base64.
        let planned_task = format!("{}{}", PLANNING_PREFIX, task);
        let first_turn = match vision {
            Some((bytes, mime)) => {
                info!("Vision mode: embedding {} bytes as {} for Gemini", bytes.len(), mime);
                let b64 = general_purpose::STANDARD.encode(&bytes);
                json!({
                    "role": "user",
                    "parts": [
                        {"text": planned_task},
                        {"inlineData": {"mimeType": mime, "data": b64}}
                    ]
                })
            }
            None => json!({"role": "user", "parts": [{"text": planned_task}]}),
        };

        let mut conversation: Vec<Value> = vec![first_turn];

        let mut final_answer: Option<String> = None;
        let mut consecutive_failures: usize = 0;
        let mut using_fallback = false;

        'agent_loop: for step in 0..self.max_steps {
            // Check for /stop between every Gemini round-trip.
            if self.cancelled.load(Ordering::SeqCst) {
                info!("Agent task cancelled by /stop request.");
                final_answer = Some("🛑 Task stopped by user.".to_string());
                break 'agent_loop;
            }

            info!("Agent step {}/{}", step + 1, self.max_steps);

            // Try primary model, escalate to fallback after repeated failures.
            let active_model = if using_fallback {
                &self.fallback_model
            } else {
                &self.model
            };
            let response = match self
                .call_gemini_model(active_model, &conversation, &system_prompt)
                .await
            {
                Ok(r) => {
                    consecutive_failures = 0;
                    r
                }
                Err(e) => {
                    consecutive_failures += 1;
                    if !using_fallback && consecutive_failures >= FALLBACK_THRESHOLD {
                        warn!(
                            "Primary model failed {} times, escalating to fallback: {}",
                            consecutive_failures, self.fallback_model
                        );
                        using_fallback = true;
                        consecutive_failures = 0;
                        // Retry immediately with fallback
                        match self
                            .call_gemini_model(&self.fallback_model, &conversation, &system_prompt)
                            .await
                        {
                            Ok(r) => r,
                            Err(e2) => return Err(e2),
                        }
                    } else {
                        return Err(e);
                    }
                }
            };
            let candidate_content = &response["candidates"][0]["content"];
            let parts: Vec<Value> = candidate_content["parts"]
                .as_array()
                .cloned()
                .unwrap_or_default();

            // — Function call branch —
            if let Some(fc_part) = parts.iter().find(|p| p.get("functionCall").is_some()) {
                let fc = &fc_part["functionCall"];
                let fn_name = fc["name"].as_str().unwrap_or("run_command");

                // Record model turn in conversation history
                conversation.push(json!({
                    "role": "model",
                    "parts": parts
                }));

                match fn_name {
                    "run_skill" => {
                        let skill_name = fc["args"]["name"].as_str().unwrap_or("").to_string();
                        let params: HashMap<String, String> = fc["args"]["params"]
                            .as_object()
                            .map(|m| {
                                m.iter()
                                    .filter_map(|(k, v)| {
                                        v.as_str().map(|s| (k.clone(), s.to_string()))
                                    })
                                    .collect()
                            })
                            .unwrap_or_default();

                        info!("Agent running skill '{}' with params: {:?}", skill_name, params);
                        Self::send_progress(
                            &progress,
                            format!("[skill] {}", skill_name),
                        )
                        .await;

                        let output = self.execute_skill(&skill_name, &params, &progress).await;
                        let (output_str, exit_code) = match output {
                            Ok(s) => (s, 0i32),
                            Err(e) => (format!("Skill error: {}", e), -1),
                        };

                        // Preprocess skill output: strip ANSI, collapse blanks, smart truncate.
                        let processed = output_utils::preprocess_output(&output_str, MAX_OUTPUT_BYTES);

                        // Structured error recovery for failed skills too.
                        let output_for_gemini = if exit_code != 0 {
                            wrap_error_reflection(&processed, exit_code)
                        } else {
                            processed
                        };

                        conversation.push(Self::fn_response("run_skill", &output_for_gemini, exit_code));
                    }

                    "web_search" => {
                        let query = fc["args"]["query"].as_str().unwrap_or("").to_string();

                        info!("Agent searching web: {}", query);
                        Self::send_progress(
                            &progress,
                            format!("[search] {}", query),
                        )
                        .await;

                        let search_timeout = Duration::from_secs(30);
                        let output = match timeout(
                            search_timeout,
                            search::search(&self.client, &query),
                        )
                        .await
                        {
                            Ok(Ok(results)) => {
                                let formatted = search::format_results(&results);
                                Self::send_progress(
                                    &progress,
                                    format!("  {} result(s)", results.len()),
                                )
                                .await;
                                formatted
                            }
                            Ok(Err(e)) => {
                                let err = format!("Search error: {}", e);
                                Self::send_progress(&progress, format!("  ✗ {}", err)).await;
                                err
                            }
                            Err(_) => {
                                let err = "Search timed out after 30s".to_string();
                                Self::send_progress(&progress, format!("  ✗ {}", err)).await;
                                err
                            }
                        };

                        let truncated = if output.len() > MAX_OUTPUT_BYTES {
                            output[..MAX_OUTPUT_BYTES].to_string()
                        } else {
                            output
                        };
                        conversation.push(Self::fn_response("web_search", &truncated, 0));
                    }

                    _ => {
                        // Default: run_command
                        let cmd = fc["args"]["command"].as_str().unwrap_or("").to_string();

                        info!("Agent running command: {}", cmd);
                        Self::send_progress(&progress, format!("$ {}", cmd)).await;

                        let cmd_timeout = Duration::from_secs(self.command_timeout);
                        let exec_result = match timeout(
                            cmd_timeout,
                            self.executor.execute(&cmd),
                        )
                        .await
                        {
                            Ok(Ok(r)) => r,
                            Ok(Err(e)) => {
                                let err = format!("Command error: {}", e);
                                Self::send_progress(&progress, format!("  ✗ {}", err)).await;
                                conversation.push(Self::fn_response("run_command", &err, -1));
                                continue;
                            }
                            Err(_) => {
                                let err = format!("Command timed out after {}s", self.command_timeout);
                                Self::send_progress(&progress, format!("  ✗ {}", err)).await;
                                conversation.push(Self::fn_response("run_command", &err, -1));
                                continue;
                            }
                        };

                        let exit_code = exec_result.exit_code;
                        let raw_output = exec_result.output();

                        let preview = if raw_output.len() > MAX_PREVIEW_BYTES {
                            format!("{}…", truncate_utf8(&raw_output, MAX_PREVIEW_BYTES))
                        } else {
                            raw_output.clone()
                        };
                        Self::send_progress(
                            &progress,
                            format!("  exit:{} {}", exit_code, preview),
                        )
                        .await;

                        // Smart output preprocessing: strip ANSI, collapse blanks,
                        // and for large outputs, extract errors first.
                        let processed = output_utils::preprocess_output(&raw_output, MAX_OUTPUT_BYTES);

                        // Structured error recovery: wrap failed commands in a
                        // reflection prompt so the model analyzes instead of retrying blindly.
                        let output_for_gemini = if exit_code != 0 {
                            wrap_error_reflection(&processed, exit_code)
                        } else {
                            processed
                        };

                        conversation
                            .push(Self::fn_response("run_command", &output_for_gemini, exit_code));
                    }
                }

            // — Text (final answer) branch —
            } else if let Some(text_part) = parts.iter().find(|p| p.get("text").is_some()) {
                let text = text_part["text"].as_str().unwrap_or("").to_string();
                info!("Agent finished after {} step(s)", step + 1);
                final_answer = Some(text);
                break 'agent_loop;
            } else {
                warn!("Agent: unexpected response: {:?}", candidate_content);
                final_answer = Some("Agent received an unexpected response format.".to_string());
                break 'agent_loop;
            }
        }

        let mut answer = final_answer.unwrap_or_else(|| {
            warn!("Agent reached max steps ({})", self.max_steps);
            format!(
                "⚠️ Reached maximum steps ({}). Task may be incomplete.",
                self.max_steps
            )
        });

        // ── Self-reflection step ──────────────────────────────────────────
        // After completing a task, ask the model to verify its own answer.
        // This catches incomplete or incorrect responses cheaply (one extra call).
        if self.enable_reflection && !self.cancelled.load(Ordering::SeqCst) {
            if let Some(reflected) = self.run_reflection(task, &answer).await {
                answer = reflected;
            }
        }

        // Persist the conversation turn to memory
        let mut needs_summary = false;
        if let Some(ref mut mem) = self.memory {
            if let Err(e) = mem
                .save_conversation(user, task, &answer, None, None, None)
                .await
            {
                warn!("Failed to save conversation to memory: {}", e);
            }
            needs_summary = mem.needs_summarization(user);
        }

        // ── Fact extraction (runs outside the memory borrow) ──────────
        if self.enable_fact_extraction {
            self.extract_and_store_facts(user, task, &answer).await;
        }

        // ── Summarization trigger ─────────────────────────────────────
        if needs_summary {
            self.generate_summary(user).await;
        }

        Ok(answer)
    }

    /// Execute all commands of a skill and return aggregated output.
    async fn execute_skill(
        &self,
        skill_name: &str,
        params: &HashMap<String, String>,
        progress: &Option<Sender<String>>,
    ) -> Result<String> {
        let sm = self
            .skill_manager
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No skill manager available"))?;

        let skill = sm
            .skills
            .get(skill_name)
            .ok_or_else(|| anyhow::anyhow!("Skill '{}' not found", skill_name))?;

        let cmd_timeout = Duration::from_secs(skill.timeout.unwrap_or(SKILL_CMD_TIMEOUT_SECS));
        let mut outputs: Vec<String> = Vec::new();
        let mut overall_exit = 0i32;

        for cmd_template in &skill.commands {
            let mut cmd = cmd_template.clone();
            // 1. Substitute skill params: {{param_name}} → value
            for (key, val) in params {
                cmd = cmd.replace(&format!("{{{{{}}}}}", key), val);
            }
            // 2. Substitute secrets: ${SECRET_NAME} → value (resolved locally,
            //    never exposed to the AI model).
            for (key, val) in &self.secrets {
                cmd = cmd.replace(&format!("${{{}}}", key), val);
            }

            Self::send_progress(progress, format!("  [skill:{}] $ {}", skill_name, cmd)).await;

            let res = match timeout(cmd_timeout, self.executor.execute(&cmd)).await {
                Ok(Ok(r)) => {
                    if !r.success() {
                        overall_exit = r.exit_code;
                    }
                    let out = r.output();
                    let preview = if out.len() > MAX_PREVIEW_BYTES {
                        format!("{}…", truncate_utf8(&out, MAX_PREVIEW_BYTES))
                    } else {
                        out.clone()
                    };
                    Self::send_progress(progress, format!("    exit:{} {}", r.exit_code, preview))
                        .await;
                    format!("$ {}\n{}", cmd, out)
                }
                Ok(Err(e)) => {
                    overall_exit = -1;
                    format!("$ {}\nError: {}", cmd, e)
                }
                Err(_) => {
                    overall_exit = -1;
                    format!("$ {}\nTimeout after {}s", cmd, cmd_timeout.as_secs())
                }
            };
            outputs.push(res);
        }

        let _ = overall_exit; // used for caller if needed
        Ok(outputs.join("\n---\n"))
    }

    // ── Helpers ────────────────────────────────────────────────────────────────

    async fn send_progress(progress: &Option<Sender<String>>, line: String) {
        if let Some(tx) = progress {
            let _ = tx.send(line).await;
        }
    }

    /// Build a functionResponse conversation turn.
    fn fn_response(name: &str, output: &str, exit_code: i32) -> Value {
        json!({
            "role": "user",
            "parts": [{"functionResponse": {
                "name": name,
                "response": {
                    "output": output,
                    "exit_code": exit_code
                }
            }}]
        })
    }

    /// Call Gemini with a specific model name.
    async fn call_gemini_model(
        &self,
        model: &str,
        conversation: &[Value],
        system_prompt: &str,
    ) -> Result<Value> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            model, self.api_key
        );

        let body = json!({
            "system_instruction": {
                "parts": [{"text": system_prompt}]
            },
            "tools": self.build_tools(),
            "contents": conversation
        });

        let resp: Value = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await?
            .json()
            .await?;

        if let Some(err) = resp.get("error") {
            return Err(anyhow::anyhow!("Gemini API error ({}): {}", model, err));
        }

        Ok(resp)
    }

    // ── Post-loop intelligence methods ────────────────────────────────────

    /// Run a self-reflection pass: ask the model to review its own answer.
    /// Returns Some(improved_answer) if the model suggests improvements,
    /// or None to keep the original.
    async fn run_reflection(&self, task: &str, answer: &str) -> Option<String> {
        let reflection_prompt = format!(
            "You just completed this task:\nTask: {}\n\nYour answer:\n{}\n\n\
             Review your answer critically. Is it complete, correct, and helpful?\n\
             If YES: respond with exactly \"LGTM\"\n\
             If NO: provide the corrected/improved answer.",
            task, answer
        );

        let conversation = vec![
            json!({"role": "user", "parts": [{"text": reflection_prompt}]}),
        ];

        let system = "You are a quality reviewer. Be concise. Only suggest changes if there are genuine issues.";

        match self.call_gemini_model(&self.model, &conversation, system).await {
            Ok(resp) => {
                let text = resp["candidates"][0]["content"]["parts"][0]["text"]
                    .as_str()
                    .unwrap_or("");
                if text.trim().to_uppercase().contains("LGTM") {
                    info!("Self-reflection: answer approved");
                    None
                } else if !text.is_empty() {
                    info!("Self-reflection: answer improved");
                    Some(text.to_string())
                } else {
                    None
                }
            }
            Err(e) => {
                warn!("Self-reflection call failed (non-critical): {}", e);
                None
            }
        }
    }

    /// Extract structured facts from a conversation turn and store them.
    async fn extract_and_store_facts(&mut self, user: &str, task: &str, answer: &str) {
        let extraction_prompt = format!(
            "Extract key facts from this conversation that should be remembered for future interactions.\n\n\
             User message: {}\nAssistant response: {}\n\n\
             Return ONLY a JSON array of objects, each with: {{\"type\": \"...\", \"key\": \"...\", \"value\": \"...\"}}\n\
             Fact types: server, preference, project, tool, credential_name (NOT the value), workflow, system_info\n\
             If no facts worth remembering, return: []\n\
             Examples:\n\
             - {{\"type\": \"server\", \"key\": \"prod\", \"value\": \"192.168.1.10 running nginx\"}}\n\
             - {{\"type\": \"preference\", \"key\": \"editor\", \"value\": \"vim\"}}\n\
             Return ONLY the JSON array, no other text.",
            task, crate::truncate_utf8(answer, 1000)
        );

        let conversation = vec![
            json!({"role": "user", "parts": [{"text": extraction_prompt}]}),
        ];

        let system = "You extract structured facts from conversations. Return only valid JSON arrays.";

        let resp = match self.call_gemini_model(&self.model, &conversation, system).await {
            Ok(r) => r,
            Err(e) => {
                warn!("Fact extraction call failed (non-critical): {}", e);
                return;
            }
        };

        let text = resp["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .unwrap_or("[]");

        // Parse the JSON response — be lenient with formatting
        let clean = text.trim().trim_start_matches("```json").trim_start_matches("```").trim_end_matches("```").trim();
        if let Ok(facts) = serde_json::from_str::<Vec<Value>>(clean) {
            if let Some(ref mut mem) = self.memory {
                for fact in facts {
                    let ft = fact["type"].as_str().unwrap_or("other");
                    let key = fact["key"].as_str().unwrap_or("");
                    let value = fact["value"].as_str().unwrap_or("");
                    if !key.is_empty() && !value.is_empty() {
                        if let Err(e) = mem.store_fact(user, ft, key, value, 0.85).await {
                            warn!("Failed to store fact: {}", e);
                        }
                    }
                }
            }
        } else {
            // Not valid JSON — that's fine, just skip
            info!("Fact extraction returned non-JSON, skipping");
        }
    }

    /// Generate a rolling summary of recent conversations and store it.
    async fn generate_summary(&mut self, user: &str) {
        // Get recent conversations to summarize
        let context = match self.memory {
            Some(ref mut mem) => mem
                .get_context(user, 20)
                .await
                .unwrap_or_default(),
            None => return,
        };

        if context.trim().is_empty() {
            return;
        }

        let summary_prompt = format!(
            "Summarize the following conversation history into a concise paragraph (3-5 sentences). \
             Focus on: what tasks were done, what the user cares about, any ongoing projects or issues.\n\n\
             {}\n\nSummary:",
            crate::truncate_utf8(&context, 3000)
        );

        let conversation = vec![
            json!({"role": "user", "parts": [{"text": summary_prompt}]}),
        ];

        let system = "You summarize conversations concisely. Focus on actionable information.";

        match self.call_gemini_model(&self.model, &conversation, system).await {
            Ok(resp) => {
                let text = resp["candidates"][0]["content"]["parts"][0]["text"]
                    .as_str()
                    .unwrap_or("");
                if !text.is_empty() {
                    if let Some(ref mem) = self.memory {
                        if let Err(e) = mem.save_summary(user, text, 20).await {
                            warn!("Failed to save summary: {}", e);
                        } else {
                            info!("Generated and saved conversation summary for {}", user);
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Summary generation failed (non-critical): {}", e);
            }
        }
    }
}
