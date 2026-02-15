"""
clide - Bot Module
Signal messenger integration for receiving and responding to user commands
"""

import os
import asyncio
from typing import Optional, Dict, Any
from signalbot import SignalBot, Command, Context
import time

from .config import Config, get_config
from .logger import get_logger, get_audit_logger
from .memory import Memory
from .brain import Brain
from .executor import Executor
from .safety import Safety, DryRun


class ClideBot:
    """Main bot orchestrator"""
    
    def __init__(self, config: Config):
        """
        Initialize clide bot
        
        Args:
            config: Configuration object
        """
        self.config = config
        self.logger = get_logger()
        self.audit = get_audit_logger()
        
        # Initialize components
        self.logger.takeoff("Initializing clide bot components")
        
        # Memory
        self.memory = Memory(
            db_path=self.config.memory.database_path,
            encryption_passphrase=self.config.memory.encryption_passphrase
        )
        
        # AI Brain
        self.brain = Brain(
            api_key=self.config.gemini.api_key,
            model=self.config.gemini.model,
            temperature=self.config.gemini.temperature,
            max_tokens=self.config.gemini.max_tokens,
            system_prompt=self.config.gemini.system_prompt
        )
        
        # Command Executor
        self.executor = Executor(
            max_retries=self.config.cline.max_retries,
            timeout=self.config.cline.timeout,
            use_cline=self.config.cline.enabled,
            cline_path=self.config.cline.cline_path
        )
        
        # Safety Checker
        self.safety = Safety(
            blocked_patterns=self.config.safety.blocked_patterns,
            requires_confirmation=self.config.safety.requires_confirmation,
            safety_level=self.config.cline.safety_level
        )
        
        # Signal bot
        self.signal_bot: Optional[SignalBot] = None
        
        # Pending confirmations (command awaiting user yes/no)
        self.pending_confirmations: Dict[str, Dict[str, Any]] = {}
        
        # Current VPS target per user
        self.current_vps: Dict[str, str] = {}
        
        self.logger.landing("Bot components initialized")
    
    def start(self) -> None:
        """Start the Signal bot"""
        self.logger.takeoff("Starting clide Signal bot")
        
        try:
            # Create Signal bot instance
            self.signal_bot = SignalBot(
                phone_number=self.config.signal.phone_number,
                # Additional signalbot config as needed
            )
            
            # Register message handler
            @self.signal_bot.handler("")
            async def handle_message(context: Context) -> None:
                await self._handle_message(context)
            
            # Start bot
            self.logger.info(f"Bot listening on: {self.config.signal.phone_number}")
            self.signal_bot.start()
            
        except Exception as e:
            self.logger.crash(f"Failed to start bot: {e}", exc_info=True)
            raise
    
    async def _handle_message(self, context: Context) -> None:
        """
        Handle incoming Signal message
        
        Args:
            context: Signal message context
        """
        user_id = context.message.source
        message_text = context.message.text
        
        # Log conversation
        self.memory.add_conversation(user_id, "user", message_text)
        
        # Check if admin only mode
        if self.config.signal.admin_only:
            if user_id not in self.config.signal.allowed_numbers:
                self.logger.warning(f"Unauthorized user: {user_id}")
                return
        
        # Check for pending confirmation
        if user_id in self.pending_confirmations:
            await self._handle_confirmation(context, user_id, message_text)
            return
        
        # Check for special commands
        if message_text.lower() in ['help', '/help']:
            await self._send_help(context)
            return
        
        if message_text.lower() in ['status', '/status']:
            await self._send_status(context, user_id)
            return
        
        if message_text.lower().startswith('switch '):
            await self._switch_vps(context, user_id, message_text)
            return
        
        # Process with AI brain
        await self._process_command(context, user_id, message_text)
    
    async def _process_command(
        self,
        context: Context,
        user_id: str,
        message: str
    ) -> None:
        """Process user command with AI"""
        self.logger.info(f"Processing: {message[:100]}...")
        
        # Build context for AI
        ai_context = self._build_context(user_id)
        
        # Get AI response
        response_text, command_data = self.brain.process_message(
            user_id,
            message,
            ai_context
        )
        
        # If no commands to execute, just respond
        if not command_data or 'commands' not in command_data:
            self.memory.add_conversation(user_id, "assistant", response_text)
            await context.send(response_text)
            return
        
        # Extract commands
        commands = command_data['commands']
        
        # Check safety for each command
        all_safe = True
        needs_confirmation = False
        safety_messages = []
        
        for cmd in commands:
            safety_result = self.safety.check_command(cmd)
            
            if not safety_result.is_safe:
                all_safe = False
                safety_messages.append(
                    f"❌ Blocked: {cmd}\n   Reason: {safety_result.reason}"
                )
            elif safety_result.requires_confirmation:
                needs_confirmation = True
        
        # If any command is blocked
        if not all_safe:
            blocked_msg = "Some commands were blocked:\n\n" + "\n".join(safety_messages)
            self.memory.add_conversation(user_id, "assistant", blocked_msg)
            await context.send(blocked_msg)
            return
        
        # If confirmation needed
        if needs_confirmation or self.config.safety.confirm_all:
            # Show dry-run preview
            preview = DryRun.preview_command(
                commands[0] if len(commands) == 1 else f"{len(commands)} commands",
                ai_context
            )
            
            # Store pending confirmation
            self.pending_confirmations[user_id] = {
                'commands': commands,
                'context': ai_context,
                'timestamp': time.time()
            }
            
            self.memory.add_conversation(user_id, "assistant", preview)
            await context.send(preview)
            return
        
        # Execute commands
        await self._execute_commands(context, user_id, commands, ai_context)
    
    async def _handle_confirmation(
        self,
        context: Context,
        user_id: str,
        response: str
    ) -> None:
        """Handle user confirmation response"""
        response_lower = response.lower().strip()
        
        if response_lower in ['yes', 'y', 'confirm', 'proceed']:
            # User confirmed - execute commands
            pending = self.pending_confirmations.pop(user_id)
            commands = pending['commands']
            ai_context = pending['context']
            
            await context.send("🛫 Executing...")
            await self._execute_commands(context, user_id, commands, ai_context)
            
        elif response_lower in ['no', 'n', 'cancel', 'abort']:
            # User cancelled
            self.pending_confirmations.pop(user_id)
            msg = "❌ Operation cancelled"
            self.memory.add_conversation(user_id, "assistant", msg)
            await context.send(msg)
            
        else:
            # Invalid response
            await context.send(
                "Please respond with 'yes' to proceed or 'no' to cancel"
            )
    
    async def _execute_commands(
        self,
        context: Context,
        user_id: str,
        commands: list,
        ai_context: Dict[str, Any]
    ) -> None:
        """Execute list of commands"""
        vps_name = ai_context.get('vps')
        ssh_config = None
        
        # Get SSH config if VPS specified
        if vps_name:
            vps_config = self.config.get_vps(vps_name)
            if vps_config:
                ssh_config = {
                    'host': vps_config.host,
                    'user': vps_config.user,
                    'port': vps_config.port,
                    'ssh_key': vps_config.ssh_key
                }
        
        # Execute each command
        results = []
        for i, cmd in enumerate(commands, 1):
            # Log command
            self.logger.log_command(cmd, "signal", user_id, vps_name)
            self.audit.log("COMMAND", user_id, cmd, vps_name or "local")
            
            # Execute
            result = self.executor.execute(
                cmd,
                vps=vps_name,
                ssh_config=ssh_config
            )
            
            results.append(result)
            
            # Log result
            self.logger.log_result(
                cmd,
                result.success,
                result.stdout,
                result.stderr,
                result.duration
            )
            
            # Store in memory
            self.memory.add_command(
                user_id,
                cmd,
                vps=vps_name,
                success=result.success,
                output=result.stdout,
                error=result.stderr,
                duration=result.duration
            )
            
            # If command failed
            if not result.success:
                error_msg = f"❌ Command {i} failed:\n{cmd}\n\nError: {result.error_message}"
                
                if result.stderr:
                    error_msg += f"\n{result.stderr[:500]}"
                
                # Try to get fix suggestion
                fix = self.brain.suggest_fix(cmd, result.stderr or result.error_message)
                if fix:
                    error_msg += f"\n\n💡 Suggested fix:\n{fix}\n\nTry this? (yes/no)"
                    
                    # Store fix as pending confirmation
                    self.pending_confirmations[user_id] = {
                        'commands': [fix],
                        'context': ai_context,
                        'timestamp': time.time()
                    }
                
                self.memory.add_conversation(user_id, "assistant", error_msg)
                await context.send(error_msg)
                return
        
        # All commands succeeded
        success_msg = self._format_success_message(commands, results)
        self.memory.add_conversation(user_id, "assistant", success_msg)
        await context.send(success_msg)
    
    def _format_success_message(
        self,
        commands: list,
        results: list
    ) -> str:
        """Format success message"""
        if len(commands) == 1:
            result = results[0]
            msg = f"🛬 Complete! ({result.duration:.2f}s)"
            
            # Include output if not too long
            if result.stdout and len(result.stdout) < 500:
                msg += f"\n\nOutput:\n{result.stdout}"
            elif result.stdout:
                msg += f"\n\nOutput (truncated):\n{result.stdout[:500]}..."
        else:
            msg = f"🛬 All {len(commands)} commands completed successfully!\n\n"
            total_duration = sum(r.duration for r in results)
            msg += f"Total time: {total_duration:.2f}s"
        
        return msg
    
    def _build_context(self, user_id: str) -> Dict[str, Any]:
        """Build context for AI"""
        context = {}
        
        # Current VPS
        if user_id in self.current_vps:
            context['vps'] = self.current_vps[user_id]
        
        # Last command
        last_cmd = self.memory.get_last_command(user_id)
        if last_cmd:
            context['last_command'] = last_cmd
        
        # User preferences
        prefs = self.memory.get_preferences(user_id)
        if prefs:
            context['preferences'] = prefs
        
        # All context
        all_context = self.memory.get_all_context(user_id)
        context.update(all_context)
        
        return context
    
    async def _switch_vps(
        self,
        context: Context,
        user_id: str,
        message: str
    ) -> None:
        """Switch target VPS"""
        # Extract VPS name
        parts = message.split(maxsplit=1)
        if len(parts) < 2:
            await context.send("Usage: switch <vps_name>")
            return
        
        vps_name = parts[1].strip()
        
        # Check if VPS exists
        vps_config = self.config.get_vps(vps_name)
        if not vps_config:
            available = [v.name for v in self.config.vps_list]
            msg = f"❌ VPS '{vps_name}' not found\n\nAvailable:\n"
            msg += "\n".join(f"  - {name}" for name in available)
            await context.send(msg)
            return
        
        # Test connection
        ssh_config = {
            'host': vps_config.host,
            'user': vps_config.user,
            'port': vps_config.port,
            'ssh_key': vps_config.ssh_key
        }
        
        success, test_msg = self.executor.test_connection(ssh_config)
        
        if success:
            self.current_vps[user_id] = vps_name
            self.memory.set_context(user_id, 'current_vps', vps_name)
            await context.send(f"✅ Switched to: {vps_name}\n{test_msg}")
        else:
            await context.send(f"❌ Cannot connect to {vps_name}\n{test_msg}")
    
    async def _send_help(self, context: Context) -> None:
        """Send help message"""
        help_text = """✈️ **clide - Help**

**Commands:**
- Just talk naturally! Ask me to do things
- `help` - Show this help
- `status` - Show bot status
- `switch <vps>` - Switch to different VPS

**Examples:**
- "What's my disk usage?"
- "Install nginx"
- "Harden my VPS to Lynis 70"
- "Setup PostgreSQL container"

**Safety:**
- Destructive operations require confirmation
- All commands are logged
- You can say 'no' to cancel

**Current Mode:** Autonomous execution with safety checks
"""
        await context.send(help_text)
    
    async def _send_status(self, context: Context, user_id: str) -> None:
        """Send bot status"""
        stats = self.memory.get_stats(user_id)
        current_vps = self.current_vps.get(user_id, "None")
        
        status = f"""📊 **clide Status**

**Your Stats:**
- Total conversations: {stats['total_conversations']}
- Commands executed: {stats['total_commands']}
- Success rate: {stats['success_rate']}%
- First interaction: {stats['first_interaction']}

**Current Target:** {current_vps}

**Available VPS:**
"""
        
        for vps in self.config.vps_list:
            status += f"  - {vps.name} ({vps.host})\n"
        
        await context.send(status)
    
    def stop(self) -> None:
        """Stop the bot"""
        self.logger.landing("Stopping clide bot")
        
        if self.signal_bot:
            self.signal_bot.stop()
        
        # Close memory connection
        self.memory.close()
        
        self.logger.info("🛬 Bot stopped gracefully")


def run_bot(config_path: str = "config.yaml") -> None:
    """
    Run the clide bot
    
    Args:
        config_path: Path to configuration file
    """
    # Load configuration
    from .config import load_config
    config = load_config(config_path)
    
    # Setup logging
    from .logger import setup_logger
    setup_logger(
        log_file=config.logging.file,
        level=config.logging.level,
        max_size=config.logging.max_size,
        backup_count=config.logging.backup_count,
        log_format=config.logging.format,
        console=config.logging.console
    )
    
    # Create and start bot
    bot = ClideBot(config)
    
    try:
        bot.start()
    except KeyboardInterrupt:
        bot.stop()
    except Exception as e:
        logger = get_logger()
        logger.crash(f"Bot crashed: {e}", exc_info=True)
        bot.stop()
        raise
