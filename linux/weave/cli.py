#!/usr/bin/env python3
"""CLI: weave new <name> <location>"""

import sys
import subprocess
import argparse
from pathlib import Path
import os


def find_script():
    """Find create_project.sh in multiple possible locations"""
    possible_paths = [
        Path(__file__).parent / "scripts" / "create_project.sh",
        Path(__file__).parent.parent / "scripts" / "create_project.sh",
        Path("/usr/local/lib/weave/scripts/create_project.sh"),
        Path("/opt/weave/scripts/create_project.sh"),
        Path.home() / ".local/lib/weave/scripts/create_project.sh",
    ]

    mayaflux_root = os.environ.get("MAYAFLUX_ROOT")
    if mayaflux_root:
        possible_paths.append(
            Path(mayaflux_root) / "share" / "weave" / "scripts" / "create_project.sh"
        )

    for path in possible_paths:
        if path.exists() and path.is_file():
            return path

    return None


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

    args = parser.parse_args()

    if args.cmd == "new":
        script = find_script()

        if not script:
            print("Error: create_project.sh not found", file=sys.stderr)
            print("Searched locations:", file=sys.stderr)
            possible = [
                Path(__file__).parent / "scripts" / "create_project.sh",
                Path(__file__).parent.parent / "scripts" / "create_project.sh",
                Path("/usr/local/lib/weave/scripts/create_project.sh"),
            ]
            for p in possible:
                print(f"  - {p}", file=sys.stderr)
            return 1

        script.chmod(0o755)

        cmd = [str(script), args.name, args.location]
        if args.with_lila:
            cmd.append("--with-lila")
        if args.no_vscode:
            cmd.append("--no-vscode")

        try:
            result = subprocess.run(cmd, check=True)
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

            app = WeaveApp()
            return app.run([])
        except ImportError:
            print("Error: GUI dependencies not installed", file=sys.stderr)
            print("Try: pip install PyGObject", file=sys.stderr)
            return 1

    else:
        parser.print_help()
        return 0


if __name__ == "__main__":
    sys.exit(main())
