#!/usr/bin/env python3
"""Verify release identity and collect evidence without building or deploying."""

import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import tomllib


ROOT = Path(__file__).resolve().parent.parent
VERSION = re.compile(r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)")
DIGEST = re.compile(r"sha256:[0-9a-f]{64}")
IMAGE = "ghcr.io/poprako-dev/poprako-server@"


def require(condition, message):
    if not condition:
        raise ValueError(message)


def run(*args):
    return subprocess.check_output(args, cwd=ROOT, text=True).strip()


def validate_snapshot(root, tag):
    version = tag.removeprefix("v")
    require(tag.startswith("v") and VERSION.fullmatch(version), "Expected vMAJOR.MINOR.PATCH")
    manifest = tomllib.loads((root / "Cargo.toml").read_text())
    require(manifest["workspace"]["package"]["version"] == version, "Tag and Cargo version differ")
    spec = json.loads((root / "docs/swagger.json").read_text())
    require(spec["info"]["version"] == version, "Tag and OpenAPI version differ")
    lock = tomllib.loads((root / "Cargo.lock").read_text())
    for member in manifest["workspace"]["members"]:
        package = tomllib.loads((root / member / "Cargo.toml").read_text())["package"]
        require(package.get("version") == {"workspace": True}, f"{member} must inherit the workspace version")
        require(any(item["name"] == package["name"] and item["version"] == version
                    and "source" not in item for item in lock["package"]),
                f"Cargo.lock has a stale version for {package['name']}")
    notes = root / "docs/releases" / f"{version}.md"
    require(notes.is_file() and notes.read_text().strip(), "Versioned release notes are required")
    require(f"## [{version}]" in (root / "CHANGELOG.md").read_text(), "CHANGELOG needs a versioned section")
    return version


def api(repository, endpoint):
    return json.loads(run("gh", "api", f"repos/{repository}/{endpoint}"))


def paginated(repository, endpoint, field):
    pages = json.loads(run("gh", "api", "--paginate", "--slurp",
                          f"repos/{repository}/{endpoint}"))
    return [item for page in pages for item in (page[field] if field else page)]


def validate_run(workflow, sha, repository):
    require(workflow["head_sha"] == sha and workflow["head_branch"] == "main",
            "CI must target the release SHA on main")
    require(workflow["event"] == "push" and workflow["conclusion"] == "success",
            "A successful main push workflow is required")
    # Review history has no attempt identity. Only a first attempt can prove
    # that its independent approval belongs to the deployment being released.
    require(workflow["run_attempt"] == 1,
            "Release evidence requires a first-attempt CI run; rerun approvals cannot be attributed")
    require(workflow["repository"]["full_name"] == repository, "Unexpected CI repository")


def validate_deployment(evidence, sha, workflow):
    require(evidence["source_commit"] == sha, "Deployment commit does not match the tag")
    require(str(evidence["run_id"]) == str(workflow["id"])
            and str(evidence["run_attempt"]) == str(workflow["run_attempt"]),
            "Deployment evidence belongs to another workflow attempt")
    require(evidence["result"] == "success", "Production deployment did not succeed")
    image = evidence["source_image"]
    require(image.startswith(IMAGE) and DIGEST.fullmatch(image.removeprefix(IMAGE)),
            "Deployment requires an immutable production image digest")
    require(evidence["local_image"] == f"poprako-server-prod:sha-{sha}", "Unexpected local image tag")
    require(DIGEST.fullmatch(evidence["image_id"]), "Deployment requires the Docker image ID")


def production_approvers(reviews, preparers):
    # Successful deployment is checked separately. The review-history API has
    # no documented chronological ordering, so do not infer a latest review.
    production = [review for review in reviews
                  if any(env["name"] == "production" for env in review["environments"])]
    require(production, "Production approval evidence is missing")
    approvers = sorted({review["user"]["login"] for review in production
                        if review["state"] == "approved" and review["user"]["login"] not in preparers})
    require(approvers, "Production approval must be independent of release preparation")
    return approvers


def collect_deployment(repository, sha, output):
    workflows = paginated(repository,
        f"actions/workflows/ci.yml/runs?head_sha={sha}&branch=main&event=push&status=success&per_page=100",
        "workflow_runs")
    require(workflows, "No successful deployed main workflow exists for the release commit")
    workflows = [workflow for workflow in workflows if workflow["run_attempt"] == 1]
    require(workflows, "No first-attempt successful CI run exists; prepare a new reviewed snapshot")
    workflow = workflows[0]
    validate_run(workflow, sha, repository)
    run_id = workflow["id"]
    jobs = paginated(repository, f"actions/runs/{run_id}/attempts/{workflow['run_attempt']}/jobs?per_page=100", "jobs")
    for name in ("Rust", "TypeScript", "Diesel migrations", "Dependency security", "HTTP integration", "Production image", "Deploy production"):
        require(any(job["name"] == name and job["conclusion"] == "success" for job in jobs),
                f"Required CI job did not succeed: {name}")
    reviews = api(repository, f"actions/runs/{run_id}/approvals")
    preparers = {workflow["actor"]["login"], workflow["triggering_actor"]["login"], os.environ["GITHUB_ACTOR"]}
    approvers = production_approvers(reviews, preparers)
    artifact_name = f"deployment-{sha}-{workflow['run_attempt']}"
    with tempfile.TemporaryDirectory(prefix="poprako-release-evidence-") as directory:
        run("gh", "run", "download", str(run_id), "--repo", repository,
            "--name", artifact_name, "--dir", directory)
        evidence = json.loads((Path(directory) / "deployment.json").read_text())
    validate_deployment(evidence, sha, workflow)
    evidence.update({"approvers": approvers, "workflow_url": workflow["html_url"],
                     "approval_history": reviews})
    output.write_text(json.dumps(evidence, indent=2) + "\n")


def validate_checkout(tag, version, sha):
    require(re.fullmatch(r"[0-9a-f]{40}", sha), "Invalid release SHA")
    require(run("git", "rev-parse", "HEAD") == sha, "Checkout does not match RELEASE_SHA")
    require(run("git", "cat-file", "-t", tag) == "tag", "An annotated release tag is required")
    require(run("git", "rev-parse", f"{tag}^{{commit}}") == sha, "Tag does not identify RELEASE_SHA")
    run("git", "merge-base", "--is-ancestor", sha, "origin/main")
    if version == "1.0.0":
        require(run("git", "rev-list", "--all", "--count") == "1",
                "1.0.0 must retain exactly one root commit across all fetched branches and tags")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tag", default=os.environ.get("RELEASE_TAG"), required="RELEASE_TAG" not in os.environ)
    parser.add_argument("--deployed", action="store_true")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    version = validate_snapshot(ROOT, args.tag)
    if args.deployed:
        require(os.environ.get("GITHUB_ACTIONS") == "true", "Deployment gate runs only in GitHub Actions")
        sha = os.environ["RELEASE_SHA"]
        validate_checkout(args.tag, version, sha)
        require(args.output is not None, "--output is required with --deployed")
        collect_deployment(os.environ["GITHUB_REPOSITORY"], sha, args.output)
    print(f"Release identity verified: {args.tag}")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, OSError, subprocess.CalledProcessError) as error:
        raise SystemExit(f"Release readiness check failed: {error}") from error
