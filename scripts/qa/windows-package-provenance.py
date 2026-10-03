"""Download a successful official Windows release candidate without rebuilding it."""

import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess


REPOSITORY = "yuxino/kiri"
WORKFLOW = ".github/workflows/build.yml"
JOB = "Build and test Windows app (Server 2025)"


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def command(*args):
    return subprocess.check_output(args, text=True, encoding="utf-8").strip()


def validate(run, jobs, run_id, tag, source_sha):
    require(run.get("id") == run_id and run.get("repository", {}).get("full_name") == REPOSITORY
            and run.get("head_repository", {}).get("full_name") == REPOSITORY,
            "Candidate must belong to the official repository")
    require(run.get("path") == WORKFLOW and run.get("event") == "push"
            and run.get("head_branch") == tag and run.get("head_sha") == source_sha,
            "Candidate must be built by the release tag's official build workflow")
    require(run.get("status") == "completed" and run.get("conclusion") == "success",
            "Candidate build must be complete and successful")
    windows = [job for job in jobs if job.get("name") == JOB]
    require(len(windows) == 1 and windows[0].get("conclusion") == "success",
            "Candidate Windows job must succeed")
    steps = {step["name"]: step.get("conclusion") for step in windows[0].get("steps", [])}
    require(all(steps.get(name) == "success" for name in (
        "Package and verify portable Windows build", "Install and smoke-test both Windows packages",
        "Upload Windows bundle")), "Candidate package and native desktop checks must succeed")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run", type=int, required=True)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--directory", type=Path, required=True)
    args = parser.parse_args()
    require(args.run > 0 and re.fullmatch(r"v\d+\.\d+\.\d+", args.tag), "Invalid candidate run or tag")
    source_sha = command("git", "rev-parse", f"refs/tags/{args.tag}^{{commit}}")
    root = f"repos/{REPOSITORY}/actions/runs/{args.run}"
    run = json.loads(command("gh", "api", root))
    jobs = json.loads(command("gh", "api", f"{root}/jobs?per_page=100"))["jobs"]
    validate(run, jobs, args.run, args.tag, source_sha)
    # A newer harness may change QA and workflow files, never application or
    # packaging sources. The downloaded installer retains its original SHA.
    harness_sha = command("git", "rev-parse", "HEAD")
    changes = command("git", "diff", "--name-only", source_sha, harness_sha).splitlines()
    require(all(path.startswith("scripts/qa/") or path.startswith(".github/workflows/")
                for path in changes), "Harness differs from candidate application/build sources")
    args.directory.mkdir(parents=True, exist_ok=True)
    subprocess.run(["gh", "run", "download", str(args.run), "--repo", REPOSITORY,
                    "--name", "kiri-windows", "--dir", str(args.directory)], check=True)
    filename = f"kiri_{args.tag[1:]}_x64-setup.exe"
    require(sorted(path.name for path in args.directory.iterdir()) == [filename, filename + ".sig"],
            "Candidate artifact must contain exactly the release installer and its signature")
    files = {path.name: {"sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
                         "size_bytes": path.stat().st_size} for path in args.directory.iterdir()}
    manifest = {"repository": REPOSITORY, "workflow_path": WORKFLOW,
                "run_id": args.run, "run_attempt": run["run_attempt"],
                "tag": args.tag, "source_sha": source_sha, "harness_sha": harness_sha,
                "harness_changes": changes, "files": files}
    (args.directory / "windows-provenance.json").write_text(
        json.dumps(manifest, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
