"""
clide - Executor Module
Handles command execution with Cline CLI integration, retry logic, and error handling
"""

import subprocess
import shlex
import time
from typing import Dict, Any, Optional, List, Tuple
from dataclasses import dataclass
import paramiko
from pathlib import Path


@dataclass
class ExecutionResult:
    """Result of command execution"""
    success: bool
    command: str
    stdout: str
    stderr: str
    return_code: int
    duration: float
    retries: int = 0
    error_message: str = ""


class Executor:
    """Command execution engine"""
    
    def __init__(
        self,
        max_retries: int = 3,
        timeout: int = 300,
        use_cline: bool = True,
        cline_path: Optional[str] = None
    ):
        """
        Initialize executor
        
        Args:
            max_retries: Maximum retry attempts on failure
            timeout: Command timeout in seconds
            use_cline: Whether to use Cline CLI for autonomous execution
            cline_path: Path to Cline executable
        """
        self.max_retries = max_retries
        self.timeout = timeout
        self.use_cline = use_cline
        self.cline_path = cline_path or self._find_cline()
    
    def _find_cline(self) -> Optional[str]:
        """Find Cline CLI in PATH"""
        try:
            result = subprocess.run(
                ['which', 'cline'],
                capture_output=True,
                text=True,
                timeout=5
            )
            if result.returncode == 0:
                return result.stdout.strip()
        except Exception:
            pass
        return None
    
    def execute(
        self,
        command: str,
        vps: Optional[str] = None,
        ssh_config: Optional[Dict[str, Any]] = None,
        retry_on_failure: bool = True,
        use_cline: Optional[bool] = None
    ) -> ExecutionResult:
        """
        Execute command locally or remotely
        
        Args:
            command: Command to execute
            vps: VPS name (if remote execution)
            ssh_config: SSH configuration for remote execution
            retry_on_failure: Whether to retry on failure
            use_cline: Override global Cline setting
            
        Returns:
            ExecutionResult
        """
        # Determine execution method
        if vps and ssh_config:
            # Remote execution via SSH
            return self._execute_remote(command, ssh_config, retry_on_failure)
        else:
            # Local execution
            use_cline_exec = use_cline if use_cline is not None else self.use_cline
            
            if use_cline_exec and self.cline_path:
                return self._execute_with_cline(command, retry_on_failure)
            else:
                return self._execute_local(command, retry_on_failure)
    
    def _execute_local(
        self,
        command: str,
        retry_on_failure: bool = True
    ) -> ExecutionResult:
        """Execute command locally using subprocess"""
        retries = 0
        start_time = time.time()
        
        while retries <= (self.max_retries if retry_on_failure else 0):
            try:
                # Execute command
                process = subprocess.Popen(
                    command,
                    shell=True,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    text=True
                )
                
                # Wait for completion with timeout
                stdout, stderr = process.communicate(timeout=self.timeout)
                return_code = process.returncode
                
                duration = time.time() - start_time
                
                # Check if successful
                if return_code == 0:
                    return ExecutionResult(
                        success=True,
                        command=command,
                        stdout=stdout,
                        stderr=stderr,
                        return_code=return_code,
                        duration=duration,
                        retries=retries
                    )
                
                # Failed - retry if enabled
                if retry_on_failure and retries < self.max_retries:
                    retries += 1
                    time.sleep(1)  # Brief delay before retry
                    continue
                
                # Max retries reached or retry disabled
                return ExecutionResult(
                    success=False,
                    command=command,
                    stdout=stdout,
                    stderr=stderr,
                    return_code=return_code,
                    duration=duration,
                    retries=retries,
                    error_message=f"Command failed with exit code {return_code}"
                )
                
            except subprocess.TimeoutExpired:
                duration = time.time() - start_time
                return ExecutionResult(
                    success=False,
                    command=command,
                    stdout="",
                    stderr="",
                    return_code=-1,
                    duration=duration,
                    retries=retries,
                    error_message=f"Command timed out after {self.timeout}s"
                )
            
            except Exception as e:
                duration = time.time() - start_time
                return ExecutionResult(
                    success=False,
                    command=command,
                    stdout="",
                    stderr=str(e),
                    return_code=-1,
                    duration=duration,
                    retries=retries,
                    error_message=f"Execution error: {str(e)}"
                )
        
        # Should not reach here, but just in case
        duration = time.time() - start_time
        return ExecutionResult(
            success=False,
            command=command,
            stdout="",
            stderr="",
            return_code=-1,
            duration=duration,
            retries=retries,
            error_message="Max retries exceeded"
        )
    
    def _execute_with_cline(
        self,
        command: str,
        retry_on_failure: bool = True
    ) -> ExecutionResult:
        """
        Execute command using Cline CLI for autonomous execution
        
        Note: This is a placeholder for Cline integration.
        Since Cline CLI may not be publicly available yet,
        we fall back to direct execution.
        """
        # TODO: Implement actual Cline CLI integration when available
        # For now, fall back to direct execution
        return self._execute_local(command, retry_on_failure)
    
    def _execute_remote(
        self,
        command: str,
        ssh_config: Dict[str, Any],
        retry_on_failure: bool = True
    ) -> ExecutionResult:
        """Execute command on remote VPS via SSH"""
        retries = 0
        start_time = time.time()
        
        while retries <= (self.max_retries if retry_on_failure else 0):
            ssh = None
            try:
                # Create SSH client
                ssh = paramiko.SSHClient()
                ssh.set_missing_host_key_policy(paramiko.AutoAddPolicy())
                
                # Connect
                connect_kwargs = {
                    'hostname': ssh_config['host'],
                    'username': ssh_config['user'],
                    'port': ssh_config.get('port', 22),
                    'timeout': 10
                }
                
                # Add authentication
                if 'ssh_key' in ssh_config and ssh_config['ssh_key']:
                    key_path = Path(ssh_config['ssh_key']).expanduser()
                    if key_path.exists():
                        connect_kwargs['key_filename'] = str(key_path)
                elif 'password' in ssh_config:
                    connect_kwargs['password'] = ssh_config['password']
                
                ssh.connect(**connect_kwargs)
                
                # Execute command
                stdin, stdout, stderr = ssh.exec_command(
                    command,
                    timeout=self.timeout
                )
                
                # Read output
                stdout_text = stdout.read().decode('utf-8')
                stderr_text = stderr.read().decode('utf-8')
                return_code = stdout.channel.recv_exit_status()
                
                duration = time.time() - start_time
                
                # Check if successful
                if return_code == 0:
                    return ExecutionResult(
                        success=True,
                        command=command,
                        stdout=stdout_text,
                        stderr=stderr_text,
                        return_code=return_code,
                        duration=duration,
                        retries=retries
                    )
                
                # Failed - retry if enabled
                if retry_on_failure and retries < self.max_retries:
                    retries += 1
                    time.sleep(1)
                    continue
                
                return ExecutionResult(
                    success=False,
                    command=command,
                    stdout=stdout_text,
                    stderr=stderr_text,
                    return_code=return_code,
                    duration=duration,
                    retries=retries,
                    error_message=f"Command failed with exit code {return_code}"
                )
                
            except paramiko.SSHException as e:
                duration = time.time() - start_time
                return ExecutionResult(
                    success=False,
                    command=command,
                    stdout="",
                    stderr=str(e),
                    return_code=-1,
                    duration=duration,
                    retries=retries,
                    error_message=f"SSH error: {str(e)}"
                )
            
            except Exception as e:
                duration = time.time() - start_time
                return ExecutionResult(
                    success=False,
                    command=command,
                    stdout="",
                    stderr=str(e),
                    return_code=-1,
                    duration=duration,
                    retries=retries,
                    error_message=f"Execution error: {str(e)}"
                )
            
            finally:
                if ssh:
                    ssh.close()
        
        # Max retries exceeded
        duration = time.time() - start_time
        return ExecutionResult(
            success=False,
            command=command,
            stdout="",
            stderr="",
            return_code=-1,
            duration=duration,
            retries=retries,
            error_message="Max retries exceeded"
        )
    
    def execute_batch(
        self,
        commands: List[str],
        vps: Optional[str] = None,
        ssh_config: Optional[Dict[str, Any]] = None,
        stop_on_error: bool = True
    ) -> List[ExecutionResult]:
        """
        Execute multiple commands in sequence
        
        Args:
            commands: List of commands to execute
            vps: VPS name (if remote)
            ssh_config: SSH configuration
            stop_on_error: Stop if a command fails
            
        Returns:
            List of ExecutionResults
        """
        results = []
        
        for command in commands:
            result = self.execute(command, vps, ssh_config)
            results.append(result)
            
            # Stop if failed and stop_on_error is True
            if not result.success and stop_on_error:
                break
        
        return results
    
    def test_connection(self, ssh_config: Dict[str, Any]) -> Tuple[bool, str]:
        """
        Test SSH connection to VPS
        
        Args:
            ssh_config: SSH configuration
            
        Returns:
            Tuple of (success, message)
        """
        ssh = None
        try:
            ssh = paramiko.SSHClient()
            ssh.set_missing_host_key_policy(paramiko.AutoAddPolicy())
            
            connect_kwargs = {
                'hostname': ssh_config['host'],
                'username': ssh_config['user'],
                'port': ssh_config.get('port', 22),
                'timeout': 10
            }
            
            if 'ssh_key' in ssh_config and ssh_config['ssh_key']:
                key_path = Path(ssh_config['ssh_key']).expanduser()
                if key_path.exists():
                    connect_kwargs['key_filename'] = str(key_path)
            
            ssh.connect(**connect_kwargs)
            
            # Test with simple command
            stdin, stdout, stderr = ssh.exec_command('echo "test"', timeout=5)
            output = stdout.read().decode('utf-8').strip()
            
            if output == "test":
                return True, f"✅ Connected to {ssh_config['host']}"
            else:
                return False, "Connection established but command test failed"
            
        except paramiko.AuthenticationException:
            return False, "❌ Authentication failed - check SSH key/password"
        
        except paramiko.SSHException as e:
            return False, f"❌ SSH error: {str(e)}"
        
        except Exception as e:
            return False, f"❌ Connection failed: {str(e)}"
        
        finally:
            if ssh:
                ssh.close()


class BackupManager:
    """Manages backups before destructive operations"""
    
    @staticmethod
    def backup_file(file_path: str, backup_dir: str = "/tmp/clide_backups") -> Optional[str]:
        """
        Create backup of file before modification
        
        Args:
            file_path: Path to file to backup
            backup_dir: Directory to store backups
            
        Returns:
            Path to backup file or None if failed
        """
        try:
            # Create backup directory
            Path(backup_dir).mkdir(parents=True, exist_ok=True)
            
            # Generate backup filename with timestamp
            timestamp = time.strftime('%Y%m%d_%H%M%S')
            file_name = Path(file_path).name
            backup_path = Path(backup_dir) / f"{file_name}.{timestamp}.backup"
            
            # Copy file
            result = subprocess.run(
                ['cp', file_path, str(backup_path)],
                capture_output=True,
                timeout=10
            )
            
            if result.returncode == 0:
                return str(backup_path)
            else:
                return None
                
        except Exception:
            return None
    
    @staticmethod
    def restore_backup(backup_path: str, original_path: str) -> bool:
        """
        Restore file from backup
        
        Args:
            backup_path: Path to backup file
            original_path: Path to restore to
            
        Returns:
            True if successful
        """
        try:
            result = subprocess.run(
                ['cp', backup_path, original_path],
                capture_output=True,
                timeout=10
            )
            return result.returncode == 0
        except Exception:
            return False


class CommandBuilder:
    """Helper class for building safe commands"""
    
    @staticmethod
    def build_safe_rm(paths: List[str], recursive: bool = False) -> str:
        """Build safe rm command with proper escaping"""
        escaped_paths = [shlex.quote(p) for p in paths]
        
        if recursive:
            return f"rm -r {' '.join(escaped_paths)}"
        else:
            return f"rm {' '.join(escaped_paths)}"
    
    @staticmethod
    def build_systemctl(action: str, service: str) -> str:
        """Build systemctl command"""
        valid_actions = ['start', 'stop', 'restart', 'reload', 'status', 'enable', 'disable']
        
        if action not in valid_actions:
            raise ValueError(f"Invalid action: {action}")
        
        return f"systemctl {action} {shlex.quote(service)}"
    
    @staticmethod
    def build_apt_install(packages: List[str], yes: bool = True) -> str:
        """Build apt install command"""
        escaped_packages = [shlex.quote(p) for p in packages]
        
        cmd = "apt install"
        if yes:
            cmd += " -y"
        cmd += f" {' '.join(escaped_packages)}"
        
        return cmd
    
    @staticmethod
    def escape_argument(arg: str) -> str:
        """Escape shell argument"""
        return shlex.quote(arg)
