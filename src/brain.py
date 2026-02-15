"""
clide - Brain Module
Gemini Flash AI integration for natural language command understanding
"""

import google.generativeai as genai
from typing import Dict, Any, List, Optional, Tuple
import json
import re


class Brain:
    """AI brain using Gemini Flash for command interpretation"""
    
    DEFAULT_SYSTEM_PROMPT = """You are clide, an autonomous terminal operator assistant.

Your role:
- Understand user requests in natural language
- Translate them into appropriate shell commands
- Execute commands safely and efficiently
- Provide clear, concise responses

Guidelines:
1. ALWAYS translate natural language to actual shell commands
2. For complex tasks, break into multiple commands
3. Use common Linux utilities (bash, apt, systemctl, etc.)
4. Be security-conscious - warn about destructive operations
5. Keep responses concise and helpful
6. Use emojis sparingly: 🛫 for starting tasks, 🛬 for completion

Output format:
- For simple queries: Direct answer
- For commands: Output as JSON with structure:
  {
    "intent": "what user wants to do",
    "commands": ["cmd1", "cmd2"],
    "explanation": "brief explanation",
    "requires_confirmation": true/false
  }

Current context will be provided in user messages.
"""
    
    def __init__(
        self,
        api_key: str,
        model: str = "gemini-2.0-flash-exp",
        temperature: float = 0.7,
        max_tokens: int = 2048,
        system_prompt: Optional[str] = None
    ):
        """
        Initialize Gemini brain
        
        Args:
            api_key: Gemini API key
            model: Model to use
            temperature: Response creativity (0.0-1.0)
            max_tokens: Max response length
            system_prompt: Custom system prompt
        """
        self.api_key = api_key
        self.model_name = model
        self.temperature = temperature
        self.max_tokens = max_tokens
        self.system_prompt = system_prompt or self.DEFAULT_SYSTEM_PROMPT
        
        # Configure Gemini
        genai.configure(api_key=self.api_key)
        
        # Initialize model
        self.model = genai.GenerativeModel(
            model_name=self.model_name,
            generation_config={
                "temperature": self.temperature,
                "max_output_tokens": self.max_tokens,
            }
        )
        
        # Conversation history
        self.chat_sessions: Dict[str, Any] = {}
    
    def process_message(
        self,
        user_id: str,
        message: str,
        context: Optional[Dict[str, Any]] = None
    ) -> Tuple[str, Optional[Dict[str, Any]]]:
        """
        Process user message and generate response
        
        Args:
            user_id: User identifier
            message: User's message
            context: Additional context (VPS, last command, etc.)
            
        Returns:
            Tuple of (response_text, parsed_command_data)
        """
        # Build prompt with context
        full_prompt = self._build_prompt(message, context)
        
        # Get or create chat session
        if user_id not in self.chat_sessions:
            self.chat_sessions[user_id] = self.model.start_chat(history=[])
        
        chat = self.chat_sessions[user_id]
        
        try:
            # Generate response
            response = chat.send_message(full_prompt)
            response_text = response.text
            
            # Try to parse as command JSON
            command_data = self._parse_command_response(response_text)
            
            if command_data:
                # Format response for user
                user_response = self._format_command_response(command_data)
                return user_response, command_data
            else:
                # Regular text response
                return response_text, None
                
        except Exception as e:
            error_msg = f"❌ AI Error: {str(e)}"
            return error_msg, None
    
    def _build_prompt(
        self,
        message: str,
        context: Optional[Dict[str, Any]] = None
    ) -> str:
        """Build full prompt with context"""
        prompt_parts = [self.system_prompt, ""]
        
        # Add context if provided
        if context:
            prompt_parts.append("CURRENT CONTEXT:")
            
            if 'vps' in context:
                prompt_parts.append(f"- Target VPS: {context['vps']}")
            
            if 'current_directory' in context:
                prompt_parts.append(f"- Current directory: {context['current_directory']}")
            
            if 'last_command' in context:
                prompt_parts.append(f"- Last command: {context['last_command']}")
            
            if 'preferences' in context:
                prefs = context['preferences']
                if 'package_manager' in prefs:
                    prompt_parts.append(f"- Preferred package manager: {prefs['package_manager']}")
            
            prompt_parts.append("")
        
        # Add user message
        prompt_parts.append(f"USER REQUEST: {message}")
        
        return '\n'.join(prompt_parts)
    
    def _parse_command_response(self, response: str) -> Optional[Dict[str, Any]]:
        """
        Try to parse response as command JSON
        
        Returns:
            Parsed command data or None
        """
        # Look for JSON in response
        json_match = re.search(r'\{.*\}', response, re.DOTALL)
        
        if json_match:
            try:
                data = json.loads(json_match.group())
                
                # Validate structure
                if 'commands' in data and isinstance(data['commands'], list):
                    return data
            except json.JSONDecodeError:
                pass
        
        return None
    
    def _format_command_response(self, command_data: Dict[str, Any]) -> str:
        """Format command data as user-friendly message"""
        lines = []
        
        # Intent
        if 'intent' in command_data:
            lines.append(f"🛫 {command_data['intent']}")
            lines.append("")
        
        # Commands
        if 'commands' in command_data:
            commands = command_data['commands']
            
            if len(commands) == 1:
                lines.append(f"Command: {commands[0]}")
            else:
                lines.append("Commands:")
                for i, cmd in enumerate(commands, 1):
                    lines.append(f"  {i}. {cmd}")
            
            lines.append("")
        
        # Explanation
        if 'explanation' in command_data:
            lines.append(command_data['explanation'])
            lines.append("")
        
        # Confirmation warning
        if command_data.get('requires_confirmation'):
            lines.append("⚠️  This operation requires confirmation")
            lines.append("Reply 'yes' to proceed or 'no' to cancel")
        
        return '\n'.join(lines)
    
    def translate_to_command(
        self,
        natural_language: str,
        context: Optional[Dict[str, Any]] = None
    ) -> List[str]:
        """
        Translate natural language to shell commands
        
        Args:
            natural_language: User's request in natural language
            context: Context information
            
        Returns:
            List of shell commands
        """
        prompt = f"""Translate this request to shell commands.
Output ONLY the commands, one per line, no explanation.

Request: {natural_language}
"""
        
        if context:
            if 'vps' in context:
                prompt += f"\nTarget: {context['vps']}"
        
        prompt += "\n\nCommands:"
        
        try:
            response = self.model.generate_content(prompt)
            commands_text = response.text.strip()
            
            # Split into individual commands
            commands = [
                cmd.strip() 
                for cmd in commands_text.split('\n') 
                if cmd.strip() and not cmd.strip().startswith('#')
            ]
            
            return commands
            
        except Exception:
            return []
    
    def explain_command(self, command: str) -> str:
        """
        Explain what a command does
        
        Args:
            command: Shell command
            
        Returns:
            Plain English explanation
        """
        prompt = f"""Explain this shell command in plain English.
Be concise (2-3 sentences).

Command: {command}

Explanation:"""
        
        try:
            response = self.model.generate_content(prompt)
            return response.text.strip()
        except Exception as e:
            return f"Could not explain command: {str(e)}"
    
    def suggest_fix(
        self,
        command: str,
        error: str,
        context: Optional[Dict[str, Any]] = None
    ) -> Optional[str]:
        """
        Suggest fix for failed command
        
        Args:
            command: Command that failed
            error: Error message
            context: Additional context
            
        Returns:
            Suggested fix command or None
        """
        prompt = f"""A command failed. Suggest a fix.

Command: {command}
Error: {error}

Provide ONLY the corrected command, no explanation.

Fixed command:"""
        
        try:
            response = self.model.generate_content(prompt)
            fixed_cmd = response.text.strip()
            
            # Remove any markdown formatting
            fixed_cmd = fixed_cmd.replace('```', '').replace('bash', '').strip()
            
            return fixed_cmd if fixed_cmd else None
            
        except Exception:
            return None
    
    def analyze_output(
        self,
        command: str,
        output: str
    ) -> str:
        """
        Analyze command output and provide insights
        
        Args:
            command: Command that was run
            output: Command output
            
        Returns:
            Analysis and insights
        """
        # Truncate very long output
        max_output = 1000
        truncated_output = output[:max_output]
        if len(output) > max_output:
            truncated_output += "\n... (output truncated)"
        
        prompt = f"""Analyze this command output and provide insights.
Be concise and highlight important information.

Command: {command}
Output:
{truncated_output}

Analysis:"""
        
        try:
            response = self.model.generate_content(prompt)
            return response.text.strip()
        except Exception as e:
            return f"Could not analyze output: {str(e)}"
    
    def get_workflow_commands(
        self,
        workflow_name: str,
        context: Optional[Dict[str, Any]] = None
    ) -> List[str]:
        """
        Generate commands for a workflow
        
        Args:
            workflow_name: Name of workflow (e.g., "harden VPS")
            context: Context information
            
        Returns:
            List of commands for workflow
        """
        prompt = f"""Generate a sequence of shell commands for this task: {workflow_name}

Output each command on a new line.
Use best practices for Linux system administration.
"""
        
        if context:
            if 'os' in context:
                prompt += f"\nOperating System: {context['os']}"
            if 'package_manager' in context:
                prompt += f"\nPackage Manager: {context['package_manager']}"
        
        prompt += "\n\nCommands:"
        
        try:
            response = self.model.generate_content(prompt)
            commands_text = response.text.strip()
            
            # Parse commands
            commands = [
                cmd.strip() 
                for cmd in commands_text.split('\n') 
                if cmd.strip() and not cmd.strip().startswith('#')
            ]
            
            return commands
            
        except Exception:
            return []
    
    def chat(
        self,
        user_id: str,
        message: str
    ) -> str:
        """
        Simple chat interface (no command parsing)
        
        Args:
            user_id: User identifier
            message: User message
            
        Returns:
            Assistant response
        """
        if user_id not in self.chat_sessions:
            self.chat_sessions[user_id] = self.model.start_chat(history=[])
        
        chat = self.chat_sessions[user_id]
        
        try:
            response = chat.send_message(message)
            return response.text
        except Exception as e:
            return f"❌ Error: {str(e)}"
    
    def reset_session(self, user_id: str) -> None:
        """Reset chat session for user"""
        if user_id in self.chat_sessions:
            del self.chat_sessions[user_id]
    
    def clear_all_sessions(self) -> None:
        """Clear all chat sessions"""
        self.chat_sessions.clear()


class CommandParser:
    """Helper class for parsing commands from AI responses"""
    
    @staticmethod
    def extract_commands(text: str) -> List[str]:
        """
        Extract shell commands from text
        
        Args:
            text: Text containing commands
            
        Returns:
            List of extracted commands
        """
        commands = []
        
        # Look for code blocks
        code_blocks = re.findall(r'```(?:bash|sh)?\n(.*?)```', text, re.DOTALL)
        
        for block in code_blocks:
            # Split by newlines
            for line in block.split('\n'):
                line = line.strip()
                # Skip comments and empty lines
                if line and not line.startswith('#'):
                    commands.append(line)
        
        # If no code blocks, look for lines that look like commands
        if not commands:
            for line in text.split('\n'):
                line = line.strip()
                # Heuristic: starts with common commands
                if line and any(line.startswith(cmd) for cmd in [
                    'sudo', 'apt', 'yum', 'systemctl', 'docker',
                    'ls', 'cd', 'cat', 'grep', 'find', 'cp', 'mv', 'rm',
                    'chmod', 'chown', 'wget', 'curl', 'git', 'npm', 'pip'
                ]):
                    commands.append(line)
        
        return commands
    
    @staticmethod
    def is_question(text: str) -> bool:
        """Check if text is a question"""
        question_words = ['what', 'how', 'why', 'when', 'where', 'who', 'which']
        text_lower = text.lower().strip()
        
        # Ends with question mark
        if text_lower.endswith('?'):
            return True
        
        # Starts with question word
        if any(text_lower.startswith(word) for word in question_words):
            return True
        
        return False
    
    @staticmethod
    def clean_command(command: str) -> str:
        """Clean and normalize command"""
        # Remove leading/trailing whitespace
        command = command.strip()
        
        # Remove markdown backticks
        command = command.replace('`', '')
        
        # Remove leading $ or # (common in examples)
        if command.startswith('$ '):
            command = command[2:]
        elif command.startswith('# '):
            command = command[2:]
        
        return command
