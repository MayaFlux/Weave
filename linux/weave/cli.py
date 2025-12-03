#!/usr/bin/env python3
"""CLI: weave new <name> <location>"""

import sys
import subprocess
import argparse
from pathlib import Path
import os

_cli_dir = Path(__file__).parent
_lib_dir = _cli_dir.parent
if str(_lib_dir) not in sys.path:
    sys.path.insert(0, str(_lib_dir))

try:
    from weave.config import get_config
except ImportError as e:
    print(
        "Error: Cannot import weave.config\n"
        f"sys.path: {sys.path}\n"
        f"Expected lib at: {_lib_dir}\n"
        f"Error: {e}",
        file=sys.stderr,
    )
    sys.exit(1)


def main():
    parser = argparse.ArgumentParser(
        prog="weave",
        description="MayaFlux project creator",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  weave new MyProject ~/Projects/
  weave new AudioApp . --with-lila
  weave gui
        """,
    )

    subparsers = parser.add_subparsers(dest="cmd", help="Commands")

    new_parser = subparsers.add_parser("new", help="Create a new project")
    new_parser.add_argument("name", help="Project name")
    new_parser.add_argument(
        "location",
        nargs="?",
        default=".",
        help="Project location (default: current directory)",
    )
    new_parser.add_argument(
        "--with-lila", action="store_true", help="Enable Lila live coding support"
    )
    new_parser.add_argument(
        "--no-vscode", action="store_true", help="Skip VS Code configuration"
    )

    gui_parser = subparsers.add_parser("gui", help="Launch graphical installer")

    parser.add_argument("--version", action="version", version="%(prog)s 0.1.0")
    parser.add_argument(
        "--config",
        action="store_true",
        help="Show configuration and exit",
    )

    args = parser.parse_args()

    if args.config:
        try:
            cfg = get_config()
            print(f"Weave Configuration")
            print(f"==================")
            print(f"Root: {cfg.root}")
            print(f"Scripts: {cfg.scripts_dir}")
            print(f"Templates: {cfg.templates_dir}")
            print(f"Python Path: {cfg.python_path}")
            print(f"\nEnvironment variables:")
            for key, value in cfg.get_env_vars().items():
                print(f"  {key}={value}")
        except Exception as e:
            print(f"Error: {e}", file=sys.stderr)
            sys.exit(1)
        return 0

    if args.cmd == "new":
        try:
            cfg = get_config()
        except Exception as e:
            print(f"Error loading configuration: {e}", file=sys.stderr)
            sys.exit(1)

        script = cfg.get_script("create_project_sh")

        if not script.exists():
            print(f"Error: create_project.sh not found at {script}", file=sys.stderr)
            sys.exit(1)

        script.chmod(0o755)

        cmd = [str(script), "new", args.name, args.location]
        if args.with_lila:
            cmd.append("--with-lila")
        if args.no_vscode:
            cmd.append("--no-vscode")

        try:
            env = os.environ.copy()
            env.update(cfg.get_env_vars())

            result = subprocess.run(cmd, env=env)
            return result.returncode
        except subprocess.CalledProcessError as e:
            print(
                f"Error: Project creation failed with exit code {e.returncode}",
                file=sys.stderr,
            )
            return e.returncode
        except FileNotFoundError:
            print(f"Error: Script not found: {script}", file=sys.stderr)
            return 1
        except Exception as e:
            print(f"Error: {e}", file=sys.stderr)
            return 1

    elif args.cmd == "gui":
        try:
            from weave.main import WeaveApp

            cfg = get_config()
            app = WeaveApp()
            return app.run([])
        except ImportError:
            print("Error: GUI dependencies not installed", file=sys.stderr)
            print("Try: pip install PyGObject", file=sys.stderr)
            return 1
        except Exception as e:
            print(f"Error: {e}", file=sys.stderr)
            return 1

    else:
        parser.print_help()
        return 0


if __name__ == "__main__":
    sys.exit(main())
