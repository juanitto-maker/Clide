// ============================================
// skills/manager.rs - Skills Manager
// ============================================
// Loads and executes modular skills from YAML files

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

use crate::executor::{ExecutionResult, Executor};

/// Skill definition from YAML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: Option<String>,
    
    #[serde(default)]
    pub tags: Vec<String>,
    
    #[serde(default)]
    pub parameters: HashMap<String, ParameterDef>,
    
    pub commands: Vec<String>,
    
    #[serde(default)]
    pub require_confirmation: bool,
    
    #[serde(default)]
    pub parallel: bool,
    
    #[serde(default)]
    pub retry_count: usize,
    
    #[serde(default)]
    pub timeout: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterDef {
    pub description: String,
    
    #[serde(rename = "type")]
    pub param_type: String,
    
    pub required: bool,
    
    #[serde(default)]
    pub default: Option<String>,
}

/// Skill execution result
#[derive(Debug)]
pub struct SkillResult {
    pub success: bool,
    pub outputs: Vec<ExecutionResult>,
    pub duration_ms: u64,
}

/// Skills manager
pub struct SkillManager {
    skills: HashMap<String, Skill>,
    skills_dir: PathBuf,
}

impl SkillManager {
    /// Create new skills manager
    pub fn new<P: AsRef<Path>>(skills_dir: P) -> Result<Self> {
        let skills_dir = skills_dir.as_ref().to_path_buf();
        
        // Create skills directory if it doesn't exist
        if !skills_dir.exists() {
            std::fs::create_dir_all(&skills_dir)?;
            info!("Created skills directory: {:?}", skills_dir);
        }

        let mut manager = Self {
            skills: HashMap::new(),
            skills_dir,
        };

        // Load all skills
        manager.load_all_skills()?;

        Ok(manager)
    }

    /// Load all skills from directory
    pub fn load_all_skills(&mut self) -> Result<()> {
        info!("Loading skills from: {:?}", self.skills_dir);

        let mut loaded_count = 0;

        // Walk through skills directory
        for entry in walkdir::WalkDir::new(&self.skills_dir)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("yaml") {
                match self.load_skill(path) {
                    Ok(skill) => {
                        info!("Loaded skill: {} v{}", skill.name, skill.version);
                        self.skills.insert(skill.name.clone(), skill);
                        loaded_count += 1;
                    }
                    Err(e) => {
                        warn!("Failed to load skill from {:?}: {}", path, e);
                    }
                }
            }
        }

        info!("Loaded {} skills", loaded_count);
        Ok(())
    }

    /// Load single skill from YAML file
    pub fn load_skill<P: AsRef<Path>>(&self, path: P) -> Result<Skill> {
        let path = path.as_ref();
        debug!("Loading skill from: {:?}", path);

        let content = std::fs::read_to_string(path)
            .context(format!("Failed to read skill file: {:?}", path))?;

        let skill: Skill = serde_yaml::from_str(&content)
            .context(format!("Failed to parse skill YAML: {:?}", path))?;

        Ok(skill)
    }

    /// Get skill by name
    pub fn get_skill(&self, name: &str) -> Option<&Skill> {
        self.skills.get(name)
    }

    /// List all skills
    pub fn list_skills(&self) -> Vec<&Skill> {
        self.skills.values().collect()
    }

    /// Search skills by tag
    pub fn find_by_tag(&self, tag: &str) -> Vec<&Skill> {
        self.skills
            .values()
            .filter(|s| s.tags.iter().any(|t| t.eq_ignore_ascii_case(tag)))
            .collect()
    }

    /// Execute a skill
    pub async fn execute_skill(
        &self,
        skill_name: &str,
        parameters: HashMap<String, String>,
        executor: &Executor,
    ) -> Result<SkillResult> {
        let skill = self
            .get_skill(skill_name)
            .ok_or_else(|| anyhow::anyhow!("Skill not found: {}", skill_name))?;

        info!("Executing skill: {}", skill.name);

        // Validate parameters
        self.validate_parameters(skill, &parameters)?;

        let start = std::time::Instant::now();
        let mut outputs = Vec::new();

        // Execute commands
        for command_template in &skill.commands {
            // Replace parameters in command
            let command = self.replace_parameters(command_template, &parameters)?;

            debug!("Executing skill command: {}", command);

            let result = executor.execute(&command).await?;
            let success = result.success();

            outputs.push(result);

            // Stop on failure unless retry is configured
            if !success && skill.retry_count == 0 {
                break;
            }
        }

        let duration_ms = start.elapsed().as_millis() as u64;
        let success = outputs.iter().all(|o| o.success());

        info!(
            "Skill execution completed: {} (success: {}, duration: {}ms)",
            skill.name, success, duration_ms
        );

        Ok(SkillResult {
            success,
            outputs,
            duration_ms,
        })
    }

    /// Validate parameters against skill definition
    fn validate_parameters(
        &self,
        skill: &Skill,
        parameters: &HashMap<String, String>,
    ) -> Result<()> {
        // Check required parameters
        for (param_name, param_def) in &skill.parameters {
            if param_def.required && !parameters.contains_key(param_name) {
                anyhow::bail!("Missing required parameter: {}", param_name);
            }
        }

        Ok(())
    }

    /// Replace parameters in command template
    fn replace_parameters(
        &self,
        template: &str,
        parameters: &HashMap<String, String>,
    ) -> Result<String> {
        let mut result = template.to_string();

        // Replace {{param_name}} with value
        for (key, value) in parameters {
            let placeholder = format!("{{{{{}}}}}", key);
            result = result.replace(&placeholder, value);
        }

        // Check for unreplaced placeholders
        if result.contains("{{") {
            warn!("Command may have unreplaced parameters: {}", result);
        }

        Ok(result)
    }

    /// Get skill as formatted string
    pub fn format_skill(&self, skill: &Skill) -> String {
        let mut output = format!(
            "📦 **{}** v{}\n{}\n",
            skill.name, skill.version, skill.description
        );

        if let Some(ref author) = skill.author {
            output.push_str(&format!("👤 Author: {}\n", author));
        }

        if !skill.tags.is_empty() {
            output.push_str(&format!("🏷️  Tags: {}\n", skill.tags.join(", ")));
        }

        if !skill.parameters.is_empty() {
            output.push_str("\n**Parameters:**\n");
            for (name, def) in &skill.parameters {
                let required = if def.required { "required" } else { "optional" };
                output.push_str(&format!("  • {}: {} ({})\n", name, def.description, required));
            }
        }

        output.push_str("\n**Commands:**\n");
        for (i, cmd) in skill.commands.iter().enumerate() {
            output.push_str(&format!("  {}. {}\n", i + 1, cmd));
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_skill_loading() {
        let dir = tempdir().unwrap();
        let skill_path = dir.path().join("test.yaml");

        let skill_yaml = r#"
name: test_skill
description: Test skill
version: "1.0.0"
commands:
  - echo "Hello {{name}}"
parameters:
  name:
    description: Name to greet
    type: string
    required: true
"#;

        std::fs::write(&skill_path, skill_yaml).unwrap();

        let manager = SkillManager::new(dir.path()).unwrap();
        assert_eq!(manager.skills.len(), 1);

        let skill = manager.get_skill("test_skill").unwrap();
        assert_eq!(skill.name, "test_skill");
        assert_eq!(skill.version, "1.0.0");
    }

    #[test]
    fn test_parameter_replacement() {
        let manager = SkillManager::new(tempdir().unwrap().path()).unwrap();
        
        let mut params = HashMap::new();
        params.insert("name".to_string(), "World".to_string());

        let result = manager
            .replace_parameters("echo Hello {{name}}", &params)
            .unwrap();

        assert_eq!(result, "echo Hello World");
    }
}