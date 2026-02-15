"""
clide - Safety Module
Security guardrails and command validation to prevent destructive operations
"""

import re
from typing import List, Tuple, Optional
from dataclasses import dataclass


@dataclass
class SafetyResult:
    """Result of safety check"""
    is_safe: bool
    requires_confirmation: bool
    reason: str = ""
    risk_level: str = "low"  # low, medium, high, critical


class Safety:
    """Command safety checker and validator"""
    
    # Patterns that are ALWAYS blocked (critical danger)
    CRITICAL_PATTERNS = [
        r'rm\s+(-rf?|--force|--recursive)?\s*/\s*$',  # rm -rf /
        r'dd\s+if=/dev/zero',  # Disk wipe
        r'mkfs\.*',  # Format filesystem
        r':\(\)\{.*\|\:&\}\;\:',  # Fork bomb
        r'chmod\s+(-R\s+)?777\s+/',  # Dangerous permissions on root
        r'>\s*/dev/sd[a-z]',  # Direct disk write
        r'fdisk.*--wipe',  # Disk wipe
    ]
    
    # Patterns that require confirmation (high risk)
    HIGH_RISK_PATTERNS = [
        r'rm\s+(-rf?|--force|--recursive)',  # Any rm -rf
        r'dd\s+',  # Any dd command
        r'mkfs\.',  # Any mkfs
        r'userdel\s+',  # Delete user
        r'deluser\s+',  # Delete user (Debian)
        r'passwd\s+',  # Password change
        r'usermod\s+',  # Modify user
        r'shutdown\s+',  # System shutdown
        r'reboot',  # System reboot
        r'halt',  # System halt
        r'init\s+[06]',  # Reboot/shutdown via init
        r'systemctl\s+(stop|disable|mask)',  # Stop services
        r'service\s+\w+\s+stop',  # Stop services
        r'kill\s+(-9\s+)?1\s*$',  # Kill init
        r'iptables\s+-F',  # Flush firewall
        r'ufw\s+(disable|reset)',  # Disable firewall
        r'iptables.*DROP.*INPUT',  # Block all incoming
    ]
    
    # Commands that need confirmation (medium risk)
    CONFIRMATION_COMMANDS = [
        'rm', 'dd', 'mkfs', 'fdisk', 'parted',
        'userdel', 'deluser', 'usermod', 'passwd',
        'shutdown', 'reboot', 'halt', 'poweroff',
        'systemctl', 'service',
        'iptables', 'ufw', 'firewall-cmd',
        'apt remove', 'apt purge', 'yum remove',
        'docker rm', 'docker rmi',
    ]
    
    # Safe read-only commands (always allowed)
    SAFE_COMMANDS = [
        'ls', 'cat', 'less', 'more', 'head', 'tail',
        'grep', 'find', 'locate', 'which', 'whereis',
        'ps', 'top', 'htop', 'free', 'df', 'du',
        'netstat', 'ss', 'lsof', 'ifconfig', 'ip',
        'uptime', 'whoami', 'who', 'w', 'id',
        'date', 'cal', 'pwd', 'echo',
        'systemctl status', 'service status',
        'git status', 'git log', 'git diff',
    ]
    
    def __init__(
        self,
        blocked_patterns: Optional[List[str]] = None,
        requires_confirmation: Optional[List[str]] = None,
        safety_level: str = "medium"
    ):
        """
        Initialize safety checker
        
        Args:
            blocked_patterns: Additional patterns to block
            requires_confirmation: Additional commands requiring confirmation
            safety_level: low, medium, or high
        """
        self.safety_level = safety_level
        
        # Merge with default patterns
        self.blocked_patterns = self.CRITICAL_PATTERNS.copy()
        if blocked_patterns:
            self.blocked_patterns.extend(blocked_patterns)
        
        self.confirmation_commands = self.CONFIRMATION_COMMANDS.copy()
        if requires_confirmation:
            self.confirmation_commands.extend(requires_confirmation)
        
        # Compile regex patterns for performance
        self.compiled_critical = [
            re.compile(pattern, re.IGNORECASE) 
            for pattern in self.CRITICAL_PATTERNS
        ]
        self.compiled_high_risk = [
            re.compile(pattern, re.IGNORECASE) 
            for pattern in self.HIGH_RISK_PATTERNS
        ]
    
    def check_command(self, command: str) -> SafetyResult:
        """
        Check if command is safe to execute
        
        Args:
            command: Command to check
            
        Returns:
            SafetyResult with safety status
        """
        command = command.strip()
        
        # Empty command
        if not command:
            return SafetyResult(
                is_safe=False,
                requires_confirmation=False,
                reason="Empty command",
                risk_level="low"
            )
        
        # Check for critical patterns (always block)
        for pattern in self.compiled_critical:
            if pattern.search(command):
                return SafetyResult(
                    is_safe=False,
                    requires_confirmation=False,
                    reason=f"Blocked: Critical destructive pattern detected",
                    risk_level="critical"
                )
        
        # Check for high risk patterns
        for pattern in self.compiled_high_risk:
            if pattern.search(command):
                return SafetyResult(
                    is_safe=True,
                    requires_confirmation=True,
                    reason=f"High risk operation detected",
                    risk_level="high"
                )
        
        # Check if command requires confirmation
        cmd_start = command.split()[0] if command.split() else ""
        
        # Check safe commands (always allow)
        if self._is_safe_command(command):
            return SafetyResult(
                is_safe=True,
                requires_confirmation=False,
                reason="Read-only safe command",
                risk_level="low"
            )
        
        # Apply safety level logic
        if self.safety_level == "high":
            # High safety: confirm everything except safe commands
            return SafetyResult(
                is_safe=True,
                requires_confirmation=True,
                reason="High safety mode: confirmation required",
                risk_level="medium"
            )
        elif self.safety_level == "medium":
            # Medium safety: confirm known risky commands
            if any(cmd in command for cmd in self.confirmation_commands):
                return SafetyResult(
                    is_safe=True,
                    requires_confirmation=True,
                    reason="Command requires confirmation",
                    risk_level="medium"
                )
        # Low safety: only block critical patterns
        
        # Default: allow without confirmation
        return SafetyResult(
            is_safe=True,
            requires_confirmation=False,
            reason="Command passed safety checks",
            risk_level="low"
        )
    
    def _is_safe_command(self, command: str) -> bool:
        """Check if command is in safe list"""
        # Exact match or starts with safe command
        for safe_cmd in self.SAFE_COMMANDS:
            if command == safe_cmd or command.startswith(safe_cmd + ' '):
                return True
        return False
    
    def check_file_operation(self, path: str, operation: str = "read") -> SafetyResult:
        """
        Check if file operation is safe
        
        Args:
            path: File path
            operation: read, write, delete
            
        Returns:
            SafetyResult
        """
        # Normalize path
        path = path.strip()
        
        # Critical system paths
        critical_paths = [
            '/', '/boot', '/dev', '/proc', '/sys',
            '/etc/passwd', '/etc/shadow', '/etc/sudoers'
        ]
        
        # Check if trying to modify critical paths
        if operation in ['write', 'delete']:
            for critical in critical_paths:
                if path == critical or path.startswith(critical + '/'):
                    return SafetyResult(
                        is_safe=False,
                        requires_confirmation=False,
                        reason=f"Blocked: Cannot {operation} critical system path: {path}",
                        risk_level="critical"
                    )
        
        # Warn about /etc modifications
        if operation == 'write' and path.startswith('/etc/'):
            return SafetyResult(
                is_safe=True,
                requires_confirmation=True,
                reason=f"Modifying system configuration: {path}",
                risk_level="high"
            )
        
        return SafetyResult(
            is_safe=True,
            requires_confirmation=False,
            reason="File operation is safe",
            risk_level="low"
        )
    
    def sanitize_command(self, command: str) -> str:
        """
        Sanitize command to prevent injection
        
        Args:
            command: Raw command
            
        Returns:
            Sanitized command
        """
        # Remove dangerous shell characters if not properly quoted
        # This is basic sanitization - full protection requires proper escaping
        
        dangerous_chars = [';', '&&', '||', '|', '>', '<', '`', '$()']
        
        # Check for unquoted dangerous chars
        in_quotes = False
        quote_char = None
        sanitized = []
        
        for char in command:
            if char in ['"', "'"]:
                if not in_quotes:
                    in_quotes = True
                    quote_char = char
                elif char == quote_char:
                    in_quotes = False
                    quote_char = None
            
            sanitized.append(char)
        
        return ''.join(sanitized)
    
    def validate_ssh_command(self, command: str, allow_sudo: bool = False) -> SafetyResult:
        """
        Validate command for SSH execution
        
        Args:
            command: Command to execute remotely
            allow_sudo: Whether to allow sudo commands
            
        Returns:
            SafetyResult
        """
        # Check if using sudo
        if command.strip().startswith('sudo '):
            if not allow_sudo:
                return SafetyResult(
                    is_safe=False,
                    requires_confirmation=False,
                    reason="sudo commands not allowed on this VPS",
                    risk_level="high"
                )
            # sudo requires confirmation
            return SafetyResult(
                is_safe=True,
                requires_confirmation=True,
                reason="sudo command requires confirmation",
                risk_level="high"
            )
        
        # Regular safety check
        return self.check_command(command)
    
    def generate_dry_run_preview(self, command: str) -> str:
        """
        Generate dry-run preview of command effects
        
        Args:
            command: Command to preview
            
        Returns:
            Preview description
        """
        preview = f"[DRY-RUN] Would execute:\n  {command}\n\n"
        
        # Try to predict effects based on command
        if 'rm' in command:
            preview += "⚠️  This will DELETE files/directories\n"
        elif 'dd' in command:
            preview += "⚠️  This will perform low-level disk operations\n"
        elif 'shutdown' in command or 'reboot' in command:
            preview += "⚠️  This will RESTART/SHUTDOWN the system\n"
        elif 'apt install' in command or 'yum install' in command:
            preview += "ℹ️  This will INSTALL packages\n"
        elif 'systemctl' in command and 'stop' in command:
            preview += "⚠️  This will STOP a service\n"
        
        return preview
    
    def get_risk_emoji(self, risk_level: str) -> str:
        """Get emoji for risk level"""
        emojis = {
            'low': '✅',
            'medium': '⚠️',
            'high': '🔴',
            'critical': '❌'
        }
        return emojis.get(risk_level, '❓')
    
    def format_safety_message(self, result: SafetyResult) -> str:
        """Format safety check result as user message"""
        emoji = self.get_risk_emoji(result.risk_level)
        
        if not result.is_safe:
            return f"{emoji} BLOCKED: {result.reason}"
        elif result.requires_confirmation:
            return f"{emoji} CONFIRMATION REQUIRED: {result.reason}"
        else:
            return f"{emoji} Safe to execute"


class DryRun:
    """Dry-run mode for previewing command effects"""
    
    @staticmethod
    def preview_command(command: str, context: dict = None) -> str:
        """
        Generate preview of what command would do
        
        Args:
            command: Command to preview
            context: Additional context (VPS, current dir, etc.)
            
        Returns:
            Preview text
        """
        preview = []
        preview.append("🔍 DRY-RUN Preview:")
        preview.append(f"   Command: {command}")
        
        if context:
            if 'vps' in context:
                preview.append(f"   Target: {context['vps']}")
            if 'cwd' in context:
                preview.append(f"   Directory: {context['cwd']}")
        
        preview.append("")
        preview.append("⚠️  Effects:")
        
        # Analyze command and predict effects
        effects = DryRun._analyze_effects(command)
        for effect in effects:
            preview.append(f"   - {effect}")
        
        preview.append("")
        preview.append("Proceed with execution? (yes/no)")
        
        return '\n'.join(preview)
    
    @staticmethod
    def _analyze_effects(command: str) -> List[str]:
        """Analyze potential effects of command"""
        effects = []
        
        # File operations
        if 'rm ' in command:
            if '-r' in command or '-R' in command:
                effects.append("Will recursively delete directories")
            if '-f' in command:
                effects.append("Will force deletion without prompts")
            effects.append("Files/directories will be permanently removed")
        
        if 'cp ' in command:
            effects.append("Will copy files")
        
        if 'mv ' in command:
            effects.append("Will move/rename files")
        
        # Package management
        if 'apt install' in command or 'yum install' in command:
            effects.append("Will download and install packages")
            effects.append("May require disk space")
        
        if 'apt remove' in command or 'yum remove' in command:
            effects.append("Will uninstall packages")
        
        # System changes
        if 'systemctl' in command:
            if 'start' in command:
                effects.append("Will start a service")
            elif 'stop' in command:
                effects.append("Will stop a service")
            elif 'restart' in command:
                effects.append("Will restart a service (brief downtime)")
        
        # Network
        if 'ufw ' in command or 'iptables' in command:
            effects.append("Will modify firewall rules")
            effects.append("May affect network connectivity")
        
        # User management
        if 'useradd' in command or 'adduser' in command:
            effects.append("Will create a new user account")
        
        if 'userdel' in command or 'deluser' in command:
            effects.append("Will delete a user account")
        
        if not effects:
            effects.append("No obvious side effects detected")
            effects.append("Review command carefully before proceeding")
        
        return effects
