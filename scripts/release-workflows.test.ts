import { describe, expect, test } from "bun:test";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { join } from "node:path";

const load = (name: string): any =>
  Bun.YAML.parse(readFileSync(new URL(`../.github/workflows/${name}.yml`, import.meta.url), "utf8"));
const preview = load("preview");
const release = load("release");
// Sourced from preview, not release: upstream anchors this step in preview.yml
// and references it from its own release.yml, but the fork replaced release.yml
// with a single-owner pipeline that has no `validate-release-source` job to
// reference it from. preview.yml is unchanged from upstream, so the gate and
// its behaviour are still covered here; the fork's own release boundaries are
// asserted separately below.
const adminGate = preview.jobs.preflight.steps[0];

describe("official publishing workflow boundaries", () => {
  test("publishing is tag-only while normal PR CI remains enabled", () => {
    expect(preview.on).toEqual({ push: { tags: ["preview-*"] } });
    // The fork publishes under its own tag namespace, so a tag inherited from
    // an upstream fetch cannot fire this pipeline on an upstream version.
    expect(release.on).toEqual({ push: { tags: ["v*-palette.[0-9]*"] } });
    expect(load("ci").on.pull_request).toBeDefined();
  });

  test("preview checks do not require a workstation Windows SDK", () => {
    const checks = preview.jobs.preflight.steps.find((step: any) => step.name === "Run checks");
    expect(checks.run.trim().split("\n")).toEqual(["just ci", "just docs-contract-test"]);
    expect(preview.jobs.build.strategy.matrix.include).toContainEqual({
      target: "x86_64-pc-windows-msvc",
      os: "windows-latest",
      name: "herdr-windows-x86_64.zip",
    });
    expect(preview.jobs.publish.needs).toContain("build");
  });

  test("each preview publishing job rechecks both actors before using credentials", () => {
    for (const name of ["preflight", "publish"]) {
      const job = preview.jobs[name];
      expect(job.if).toContain("github.event_name == 'push'");
      expect(job.if).toContain("startsWith(github.ref, 'refs/tags/");
      expect(job.steps[0]).toEqual(adminGate);
    }
    expect(adminGate.run).toContain('"$GITHUB_ACTOR" "$GITHUB_TRIGGERING_ACTOR"');
    expect(adminGate.env.GH_TOKEN).toBe("${{ github.token }}");
    expect(adminGate.run).not.toContain("ogulcancelik");
  });

  // The fork's release.yml is its own pipeline (build/release/tap-bump), not
  // upstream's. It carries no multi-admin gate because it has one owner, so
  // what is worth pinning here is least privilege: the default is read, each
  // job widens only to what it needs, and the one credential that reaches
  // another repository is scoped to that repository.
  test("the fork's release pipeline grants the narrowest permissions it can", () => {
    expect(release.permissions).toEqual({ contents: "read" });
    expect(release.jobs.release.permissions).toEqual({ contents: "write" });
    // No `contents` at all: nothing in tap-bump writes to this repository.
    expect(release.jobs["tap-bump"].permissions).toEqual({ actions: "read" });

    const tapToken = release.jobs["tap-bump"].steps.find(
      (step: any) => step.id === "tap-token",
    );
    expect(tapToken.uses).toContain("actions/create-github-app-token@");
    expect(tapToken.with.repositories).toBe("homebrew-tap");
    expect(tapToken.with["permission-contents"]).toBe("write");
    // A private key belongs in secrets; an App id is not a secret but must not
    // be inlined either, or rotating it means editing the workflow.
    expect(tapToken.with["private-key"]).toBe("${{ secrets.RELEASE_APP_PRIVATE_KEY }}");
    expect(tapToken.with["client-id"]).toBe("${{ vars.RELEASE_APP_ID }}");
  });

  test("release arguments are not interpolated into executable shell text", () => {
    const input = `untrusted'\"$(echo unexpected-command)`;
    for (const args of [
      ["preview", input],
      ["release-prepare", input, input],
      ["release-publish", input, input],
      ["release", input, input],
    ]) {
      const result = spawnSync("just", ["--dry-run", ...args], { encoding: "utf8" });
      expect(result.status).toBe(0);
      expect(result.stdout + result.stderr).not.toContain(input);
      expect(result.stdout + result.stderr).not.toContain("unexpected-command");
    }
  });

  test.skipIf(process.platform === "win32")("admin gate permits admins and fails closed for other roles or API errors", () => {
    const dir = mkdtempSync("/var/tmp/herdr-admin-gate-");
    try {
      writeFileSync(join(dir, "gh"), `#!/bin/sh
case "$2" in
  */collaborators/admin-*/permission) echo admin ;;
  */collaborators/maintainer/permission) echo maintain ;;
  */collaborators/writer/permission) echo write ;;
  *) exit 1 ;;
esac
`, { mode: 0o755 });
      for (const [actor, trigger, succeeds] of [
        ["admin-one", "admin-two", true],
        ["writer", "admin-two", false],
        ["admin-one", "writer", false],
        ["admin-one", "maintainer", false],
        ["admin-one", "api-error", false],
      ] as const) {
        const result = spawnSync("bash", ["-c", adminGate.run], {
          env: { ...process.env, PATH: `${dir}:${process.env.PATH}`, GITHUB_REPOSITORY: "example/test", GITHUB_ACTOR: actor, GITHUB_TRIGGERING_ACTOR: trigger },
          encoding: "utf8",
        });
        expect(result.status === 0).toBe(succeeds);
      }
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });
});
