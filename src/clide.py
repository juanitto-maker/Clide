#!/usr/bin/env python3
"""
clide - Main Entry Point
Glide through your CLI - Autonomous terminal operations from your pocket
"""

import sys
import os
import argparse
from pathlib import Path

# Add src directory to path
sys.path.insert(0, str(Path(__file__).parent))

from config import load_config, Config
from logger import setup_logger, get_logger
from bot import run_bot


ASCII_ART = """
    ✈️  ════════════════════════════════════ ✈️
    
         _____ _      _____ _____  ______ 
        /  ___| |    |_   _|  _  \\ |  ___|
        | |   | |      | | | | | || |__   
        | |   | |      | | | | | ||  __|  
        \\ \\___| |____ _| |_| |/ / | |___  
         \\____|______|\\_____/|___/|______|
    
        Glide through your CLI
        Autonomous terminal operations
    
    ✈️  ════════════════════════════════════ ✈️
"""


def print_banner():
    """Print clide banner"""
    print(ASCII_ART)
    print("    Version: 0.1.0-alpha")
    print("    https://github.com/yourusername/clide")
    print()


def check_dependencies():
    """Check if required dependencies are installed"""
    missing = []
    
    try:
        import yaml
    except ImportError:
        missing.append("pyyaml")
    
    try:
        import google.generativeai
    except ImportError:
        missing.append("google-generativeai")
    
    try:
        from signalbot import SignalBot
    except ImportError:
        missing.append("signalbot")
    
    try:
        import paramiko
    except ImportError:
        missing.append("paramiko")
    
    try:
        from cryptography.fernet import Fernet
    except ImportError:
        missing.append("cryptography")
    
    if missing:
        print("❌ Missing dependencies:")
        for dep in missing:
            print(f"   - {dep}")
        print()
        print("Install with: pip install -r requirements.txt")
        sys.exit(1)


def verify_config(config_path: str = "config.yaml") -> bool:
    """Verify configuration file exists and is valid"""
    if not os.path.exists(config_path):
        print(f"❌ Configuration file not found: {config_path}")
        print()
        print("Create it from the example:")
        print(f"  cp config.example.yaml {config_path}")
        print(f"  nano {config_path}")
        print()
        return False
    
    try:
        config = load_config(config_path)
        
        # Check critical settings
        warnings = []
        
        if not config.signal.phone_number or config.signal.phone_number == "+1234567890":
            warnings.append("Signal phone number not configured")
        
        if not config.gemini.api_key or config.gemini.api_key == "YOUR_GEMINI_API_KEY_HERE":
            warnings.append("Gemini API key not configured")
        
        if warnings:
            print("⚠️  Configuration warnings:")
            for warning in warnings:
                print(f"   - {warning}")
            print()
            print(f"Please edit {config_path} and configure these settings.")
            print()
            return False
        
        return True
        
    except Exception as e:
        print(f"❌ Configuration error: {e}")
        print()
        return False


def setup_first_run():
    """Interactive setup wizard for first run"""
    print("🛫 First-time setup wizard")
    print()
    
    # Check if config exists
    if not os.path.exists("config.yaml"):
        print("Creating config.yaml from template...")
        
        if os.path.exists("config.example.yaml"):
            import shutil
            shutil.copy("config.example.yaml", "config.yaml")
            print("✅ Created config.yaml")
        else:
            print("❌ config.example.yaml not found")
            print("Please download it from the repository")
            return False
    
    print()
    print("📝 Next steps:")
    print("1. Edit config.yaml with your settings:")
    print("   - Signal phone number")
    print("   - Gemini API key")
    print("   - VPS configurations (optional)")
    print()
    print("2. Link Signal account:")
    print("   signal-cli link -n 'clide-bot'")
    print()
    print("3. Get Gemini API key:")
    print("   https://makersuite.google.com/app/apikey")
    print()
    print("4. Run clide again:")
    print("   python src/clide.py")
    print()
    
    return False


def cmd_start(args):
    """Start clide bot"""
    print_banner()
    
    # Check dependencies
    check_dependencies()
    
    # Verify configuration
    if not verify_config(args.config):
        if args.setup:
            setup_first_run()
        return 1
    
    # Start bot
    print("🛫 Starting clide...")
    print()
    
    try:
        run_bot(args.config)
    except KeyboardInterrupt:
        print()
        print("🛬 Shutting down gracefully...")
        return 0
    except Exception as e:
        print(f"❌ Fatal error: {e}")
        return 1


def cmd_test(args):
    """Test configuration and connections"""
    print_banner()
    print("🔍 Testing configuration...")
    print()
    
    # Load config
    try:
        config = load_config(args.config)
    except Exception as e:
        print(f"❌ Configuration error: {e}")
        return 1
    
    # Test 1: Configuration
    print("1️⃣  Configuration file... ✅")
    print()
    
    # Test 2: Gemini API
    print("2️⃣  Testing Gemini API...")
    try:
        from brain import Brain
        brain = Brain(
            api_key=config.gemini.api_key,
            model=config.gemini.model
        )
        response = brain.chat("test", "Say 'test successful'")
        if "success" in response.lower():
            print("   ✅ Gemini API working")
        else:
            print("   ⚠️  Gemini API responded but unclear result")
    except Exception as e:
        print(f"   ❌ Gemini API error: {e}")
    print()
    
    # Test 3: Signal CLI
    print("3️⃣  Testing signal-cli...")
    try:
        import subprocess
        result = subprocess.run(
            ['signal-cli', '--version'],
            capture_output=True,
            timeout=5
        )
        if result.returncode == 0:
            version = result.stdout.decode().strip()
            print(f"   ✅ signal-cli found: {version}")
        else:
            print("   ❌ signal-cli not working")
    except FileNotFoundError:
        print("   ❌ signal-cli not installed")
    except Exception as e:
        print(f"   ⚠️  signal-cli test failed: {e}")
    print()
    
    # Test 4: VPS connections
    if config.vps_list:
        print("4️⃣  Testing VPS connections...")
        from executor import Executor
        executor = Executor()
        
        for vps in config.vps_list:
            ssh_config = {
                'host': vps.host,
                'user': vps.user,
                'port': vps.port,
                'ssh_key': vps.ssh_key
            }
            
            success, msg = executor.test_connection(ssh_config)
            print(f"   {vps.name}: {msg}")
        print()
    
    # Test 5: Memory database
    print("5️⃣  Testing memory database...")
    try:
        from memory import Memory
        memory = Memory(config.memory.database_path)
        memory.close()
        print("   ✅ Database initialized")
    except Exception as e:
        print(f"   ❌ Database error: {e}")
    print()
    
    print("🛬 Tests complete!")
    return 0


def cmd_version(args):
    """Show version information"""
    print_banner()
    
    print("Dependencies:")
    
    packages = [
        'yaml', 'google.generativeai', 'signalbot',
        'paramiko', 'cryptography'
    ]
    
    for pkg in packages:
        try:
            mod = __import__(pkg)
            version = getattr(mod, '__version__', 'unknown')
            print(f"  ✅ {pkg}: {version}")
        except ImportError:
            print(f"  ❌ {pkg}: not installed")
    
    print()
    return 0


def cmd_cleanup(args):
    """Cleanup old data"""
    print("🧹 Cleaning up old data...")
    
    try:
        config = load_config(args.config)
        from memory import Memory
        
        memory = Memory(config.memory.database_path)
        
        days = args.days or config.memory.cleanup_days
        conv_deleted, cmd_deleted = memory.cleanup_old_data(days)
        
        print(f"✅ Deleted:")
        print(f"   - {conv_deleted} old conversations")
        print(f"   - {cmd_deleted} old commands")
        
        if args.vacuum:
            print()
            print("🗜️  Optimizing database...")
            memory.vacuum()
            print("✅ Database optimized")
        
        memory.close()
        
    except Exception as e:
        print(f"❌ Cleanup failed: {e}")
        return 1
    
    return 0


def main():
    """Main entry point"""
    parser = argparse.ArgumentParser(
        description="clide - Glide through your CLI",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  python src/clide.py                    # Start bot
  python src/clide.py --setup            # First-time setup
  python src/clide.py test               # Test configuration
  python src/clide.py cleanup --days 30  # Clean old data
        """
    )
    
    parser.add_argument(
        '-c', '--config',
        default='config.yaml',
        help='Configuration file (default: config.yaml)'
    )
    
    parser.add_argument(
        '--setup',
        action='store_true',
        help='Run first-time setup wizard'
    )
    
    subparsers = parser.add_subparsers(dest='command', help='Commands')
    
    # Start command (default)
    start_parser = subparsers.add_parser('start', help='Start the bot')
    start_parser.set_defaults(func=cmd_start)
    
    # Test command
    test_parser = subparsers.add_parser('test', help='Test configuration')
    test_parser.set_defaults(func=cmd_test)
    
    # Version command
    version_parser = subparsers.add_parser('version', help='Show version')
    version_parser.set_defaults(func=cmd_version)
    
    # Cleanup command
    cleanup_parser = subparsers.add_parser('cleanup', help='Clean old data')
    cleanup_parser.add_argument(
        '--days',
        type=int,
        help='Delete data older than N days'
    )
    cleanup_parser.add_argument(
        '--vacuum',
        action='store_true',
        help='Optimize database after cleanup'
    )
    cleanup_parser.set_defaults(func=cmd_cleanup)
    
    args = parser.parse_args()
    
    # Default to start command
    if not args.command:
        args.func = cmd_start
    
    # Execute command
    try:
        return args.func(args)
    except AttributeError:
        parser.print_help()
        return 1


if __name__ == '__main__':
    sys.exit(main())
