// ============================================
// agent.rs - Autonomous Agentic Loop
// ============================================
// Uses Gemini function-calling to iteratively run shell commands
// until the model produces a final text answer.

use anyhow::Result;
use log::{info, warn};
use reqwest::Client;
use serde_json::{json, Value};
use tokio::sync::mpsc::Sender;
use tokio::time::{timeout, Duration};

use crate::config::Config;
use crate::executor::Executor;

/// Maximum output bytes fed back to Gemini per command (avoids context overflow)
const MAX_OUTPUT_BYTES: usize = 8_000;
/// Maximum progress preview sent over the channel per command
const MAX_PREVIEW_BYTES: usize = 500;

const SYSTEM_PROMPT: &str = "\
You are Clide, an autonomous CLI operator running inside a Termux terminal on Android. \
You have direct shell access via the `run_command` tool.\n\n\
Your capabilities:\n\
- Execute any shell command (bash, python, node, etc.)\n\
- Install packages with pkg / apt / pip / npm\n\
- Create, read, and edit files\n\
- Set up cron jobs with crontab\n\
- Run background processes with nohup / screen / tmux\n\
- Access the internet with curl / wget\n\n\
Your approach:\n\
1. Break the task into concrete steps.\n\
2. Execute each step immediately using run_command — do NOT describe or explain first.\n\
3. Inspect results and adapt if something fails.\n\
4. When finished, give a concise summary of what was accomplished.\n\n\
IMPORTANT: Always use run_command to get information or take action. \
Never respond with 'I would do X' — just do it.";

pub struct Agent {
    client: Client,
    api_key: String,
    model: String,
    executor: Executor,
    max_steps: usize,
}

impl Agent {
    pub fn new(config: &Config) -> Self {
        Self {
            client: Client::new(),
            api_key: config.gemini_api_key.clone(),
            model: config.get_model().to_string(),
            executor: Executor::new(config.clone()),
            max_steps: config.max_agent_steps,
        }
    }

    /// Run the agentic task loop.
    ///
    /// Sends incremental progress strings via `progress` (if provided).
    /// Returns the final text answer from the model.
    pub async fn run(&mut self, task: &str, progress: Option<Sender<String>>) -> Result<String> {
        info!("Agent starting task: {}", task);

        let mut conversation: Vec<Value> =
            vec![json!({"role": "user", "parts": [{"text": task}]})];

        for step in 0..self.max_steps {
            info!("Agent step {}/{}", step + 1, self.max_steps);

            let response = self.call_gemini(&conversation).await?;
            let candidate_content = &response["candidates"][0]["content"];
            let parts: Vec<Value> = candidate_content["parts"]
                .as_array()
                .cloned()
                .unwrap_or_default();

            // — Function call branch —
            if let Some(fc_part) = parts.iter().find(|p| p.get("functionCall").is_some()) {
                let fc = &fc_part["functionCall"];
                let cmd = fc["args"]["command"].as_str().unwrap_or("").to_string();

                info!("Agent running command: {}", cmd);

                if let Some(ref tx) = progress {
                    let _ = tx.send(format!("$ {}", cmd)).await;
                }

                // Record model turn in conversation history
                conversation.push(json!({
                    "role": "model",
                    "parts": parts
                }));

                // Execute with 60-second timeout
                let exec_result =
                    match timeout(Duration::from_secs(60), self.executor.execute(&cmd)).await {
                        Ok(Ok(r)) => r,
                        Ok(Err(e)) => {
                            let err = format!("Command error: {}", e);
                            Self::send_progress(&progress, format!("  ✗ {}", err)).await;
                            conversation.push(Self::fn_response("run_command", &err, -1));
                            continue;
                        }
                        Err(_) => {
                            let err = "Command timed out after 60s".to_string();
                            Self::send_progress(&progress, format!("  ✗ {}", err)).await;
                            conversation.push(Self::fn_response("run_command", &err, -1));
                            continue;
                        }
                    };

                let exit_code = exec_result.exit_code;
                let output = exec_result.output();

                // Progress preview (capped)
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

                // Feed truncated output back to Gemini
                let output_for_gemini = if output.len() > MAX_OUTPUT_BYTES {
                    output[..MAX_OUTPUT_BYTES].to_string()
                } else {
                    output
                };
                conversation.push(Self::fn_response("run_command", &output_for_gemini, exit_code));

            // — Text (final answer) branch —
            } else if let Some(text_part) = parts.iter().find(|p| p.get("text").is_some()) {
                let text = text_part["text"].as_str().unwrap_or("").to_string();
                info!("Agent finished after {} step(s)", step + 1);
                return Ok(text);

            } else {
                warn!("Agent: unexpected response: {:?}", candidate_content);
                return Ok("Agent received an unexpected response format.".to_string());
            }
        }

        warn!("Agent reached max steps ({})", self.max_steps);
        Ok(format!(
            "⚠️ Reached maximum steps ({}). Task may be incomplete.",
            self.max_steps
        ))
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
    async fn call_gemini(&self, conversation: &[Value]) -> Result<Value> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );

        let body = json!({
            "system_instruction": {
                "parts": [{"text": SYSTEM_PROMPT}]
            },
            "tools": [{
                "function_declarations": [{
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
                }]
            }],
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
