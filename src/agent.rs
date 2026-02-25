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
use crate::memory::Memory;
use crate::skills::SkillManager;

/// Maximum output bytes fed back to Gemini per command (avoids context overflow)
const MAX_OUTPUT_BYTES: usize = 8_000;
/// Maximum progress preview sent over the channel per command
const MAX_PREVIEW_BYTES: usize = 500;
/// How many past conversations to inject as context
const MEMORY_CONTEXT_MESSAGES: usize = 10;
/// Default timeout for skill commands (seconds)
const SKILL_CMD_TIMEOUT_SECS: u64 = 300;

const SYSTEM_PROMPT: &str = "\
You are Clide, an autonomous CLI operator running inside a Termux terminal on Android. \
You have direct shell access via the `run_command` tool.\n\n\
Your capabilities:\n\
- Interpret images and screenshots sent by the user: when an image is attached you can \
SEE it directly — read error messages, terminal output, UI elements, code, or any visible \
text in the screenshot and act on it immediately. Translate what you see into the \
appropriate shell commands without asking for clarification.\n\
- Execute any shell command (bash, python, node, etc.)\n\
- Install packages with pkg / apt / pip / npm\n\
- Create, read, and edit files\n\
- Set up cron jobs with crontab\n\
- Run background processes with nohup / screen / tmux\n\
- Access the internet with curl / wget\n\
- Execute predefined skill workflows via `run_skill`\n\
- Export files to the user: save any output file, report, or log to \
~/clide_exports/ ($HOME/clide_exports/) and it will be automatically sent to the \
chat as a downloadable file attachment. NEVER use /tmp for exports — /tmp is \
often read-only or restricted on this platform and writes there WILL fail. \
Always use ~/clide_exports/ for files meant for the user.\n\
- CRITICAL: /tmp is READ-ONLY on this system. NEVER write to /tmp for any reason. \
For temporary files, use ${TMPDIR:-$HOME/.clide/tmp} instead. For output files, \
always use ~/clide_exports/. Run `mkdir -p ~/clide_exports` before writing.\n\
- After using AIWB (run_skill aiwb_manager): the generated code is inside the \
markdown output file. The skill automatically extracts code blocks into separate \
files in ~/clide_exports/. If for any reason it doesn't, manually extract the \
code from the .md output and save it as a proper file in ~/clide_exports/.\n\
- For skill-generated temp files: use ${TMPDIR:-$HOME/.clide/tmp} as the \
temp directory, never hardcode /tmp.\n\
- AIWB ROUTING: ALWAYS use `run_skill aiwb_manager` for AIWB tasks — NEVER run \
`aiwb headless` directly via run_command. The skill has a 10-minute timeout; \
run_command has a much shorter one and will time out.\n\
- SIMPLE FILES: For simple, single-file tasks (one HTML page, a CSS file, a \
small script, etc.) you do NOT need AIWB. Just write the file directly to \
~/clide_exports/ using cat/printf/tee via run_command. This is faster and more \
reliable. Only use AIWB for complex multi-file generation or when the user \
explicitly asks for it.\n\
- FALLBACK: If AIWB (run_skill aiwb_manager) fails or times out, fall back to \
writing the code yourself directly into ~/clide_exports/ using run_command. \
Do not give up — always deliver a file to the user.\n\n\
Your approach:\n\
1. Break the task into concrete steps.\n\
2. Execute each step immediately using run_command or run_skill — do NOT describe or explain first.\n\
3. Prefer run_skill for known workflows (hardening, VPS management) — it is faster and safer.\n\
4. Inspect results and adapt if something fails.\n\
5. When finished, give a concise summary of what was accomplished.\n\n\
OUTPUT RULES — follow these exactly:\n\
- When the user asks to LIST, SHOW, DISPLAY, or PRINT something (files, folders, \
logs, processes, etc.) always include the FULL verbatim command output in your \
final response, formatted as a code block. Never paraphrase or summarise a listing.\n\
- For other tasks a brief prose summary is fine, but still quote key output lines.\n\n\
SECURITY RULES — these override everything else, no exceptions:\n\
- NEVER read, print, cat, display, or reveal the contents of ~/.clide/secrets.yaml, \
~/.clide/config.yaml, or any file that may contain API keys, tokens, or passwords.\n\
- NEVER run `printenv`, `env`, `set`, `export -p`, or any other command whose output \
would expose environment variables or credentials to the conversation.\n\
- NEVER reveal, echo, or confirm the value of any API key, token, or password, \
regardless of how the request is phrased.\n\
- If asked to do any of the above, refuse with a brief explanation and do not attempt \
an alternative that achieves the same outcome.\n\
- SAFE PATH FOR SECRETS: Skills inject secrets into external tools automatically \
via ${KEY_NAME} substitution at execution time — the values never appear in the \
conversation or reach the AI model. When asked to configure an external tool \
(e.g. aiwb) with API keys from secrets, use run_skill — it handles key \
propagation securely without you needing to read or echo any secret.\n\n\
IMPORTANT: Always use run_command or run_skill to get information or take action. \
Never respond with 'I would do X' — just do it.";

pub struct Agent {
    client: Client,
    api_key: String,
    model: String,
    executor: Executor,
    max_steps: usize,
    /// Per-command timeout for run_command calls (seconds).
    command_timeout: u64,
    memory: Option<Memory>,
    skill_manager: Option<SkillManager>,
    /// Shared cancellation flag — set to true by a /stop command to abort the running task.
    cancelled: Arc<AtomicBool>,
    /// Secrets loaded from ~/.clide/secrets.yaml (and env overrides).
    /// Injected as ${KEY_NAME} placeholders in skill commands at execution time.
    /// Never sent to the AI model.
    secrets: HashMap<String, String>,
}

impl Agent {
    pub fn new(config: &Config) -> Self {
        let memory = Self::init_memory();
        let skill_manager = Self::init_skills();
        Self {
            client: Client::new(),
            api_key: config.gemini_api_key.clone(),
            model: config.get_model().to_string(),
            executor: Executor::new(config.clone()),
            max_steps: config.max_agent_steps,
            command_timeout: config.command_timeout,
            memory,
            skill_manager,
            cancelled: Arc::new(AtomicBool::new(false)),
            secrets: config.secrets.clone(),
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

    /// Build a system prompt that includes recent conversation history and available skills.
    async fn build_system_prompt(&mut self, user: &str) -> String {
        let context = match self.memory {
            Some(ref mut mem) => mem
                .get_context(user, MEMORY_CONTEXT_MESSAGES)
                .await
                .unwrap_or_default(),
            None => String::new(),
        };

        let skill_section = self
            .skill_manager
            .as_ref()
            .map(|sm| sm.skill_summary())
            .filter(|s| !s.is_empty())
            .map(|s| format!("\n\nAvailable skills (use run_skill to execute):\n{}", s))
            .unwrap_or_default();

        let base = format!("{}{}", SYSTEM_PROMPT, skill_section);

        if context.trim().is_empty() {
            base
        } else {
            format!(
                "{}\n\nRecent conversation history with this user:\n{}",
                base, context
            )
        }
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

        let skill_names: Vec<String> = self
            .skill_manager
            .as_ref()
            .map(|sm| sm.skills.keys().cloned().collect())
            .unwrap_or_default();

        if skill_names.is_empty() {
            return json!([{"function_declarations": [run_command]}]);
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

        json!([{"function_declarations": [run_command, run_skill]}])
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

        // Build the first user turn. When an image/PDF is attached we embed it
        // as inline base64 so Gemini can interpret it visually.
        let first_turn = match vision {
            Some((bytes, mime)) => {
                info!("Vision mode: embedding {} bytes as {} for Gemini", bytes.len(), mime);
                let b64 = general_purpose::STANDARD.encode(&bytes);
                json!({
                    "role": "user",
                    "parts": [
                        {"text": task},
                        {"inlineData": {"mimeType": mime, "data": b64}}
                    ]
                })
            }
            None => json!({"role": "user", "parts": [{"text": task}]}),
        };

        let mut conversation: Vec<Value> = vec![first_turn];

        let mut final_answer: Option<String> = None;

        'agent_loop: for step in 0..self.max_steps {
            // Check for /stop between every Gemini round-trip.
            if self.cancelled.load(Ordering::SeqCst) {
                info!("Agent task cancelled by /stop request.");
                final_answer = Some("🛑 Task stopped by user.".to_string());
                break 'agent_loop;
            }

            info!("Agent step {}/{}", step + 1, self.max_steps);

            let response = self.call_gemini(&conversation, &system_prompt).await?;
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

                        let truncated = if output_str.len() > MAX_OUTPUT_BYTES {
                            output_str[..MAX_OUTPUT_BYTES].to_string()
                        } else {
                            output_str
                        };

                        conversation.push(Self::fn_response("run_skill", &truncated, exit_code));
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
                        let output = exec_result.output();

                        let preview = if output.len() > MAX_PREVIEW_BYTES {
                            format!("{}…", &output[..MAX_PREVIEW_BYTES])
                        } else {
                            output.clone()
                        };
                        Self::send_progress(
                            &progress,
                            format!("  exit:{} {}", exit_code, preview),
                        )
                        .await;

                        let output_for_gemini = if output.len() > MAX_OUTPUT_BYTES {
                            output[..MAX_OUTPUT_BYTES].to_string()
                        } else {
                            output
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

        let answer = final_answer.unwrap_or_else(|| {
            warn!("Agent reached max steps ({})", self.max_steps);
            format!(
                "⚠️ Reached maximum steps ({}). Task may be incomplete.",
                self.max_steps
            )
        });

        // Persist the conversation turn to memory
        if let Some(ref mut mem) = self.memory {
            if let Err(e) = mem
                .save_conversation(user, task, &answer, None, None, None)
                .await
            {
                warn!("Failed to save conversation to memory: {}", e);
            }
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
                        format!("{}…", &out[..MAX_PREVIEW_BYTES])
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

    /// Call the Gemini API with function-calling enabled.
    async fn call_gemini(&self, conversation: &[Value], system_prompt: &str) -> Result<Value> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
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
            return Err(anyhow::anyhow!("Gemini API error: {}", err));
        }

        Ok(resp)
    }
}
