# herdr (fork)

A fork of [herdrdev/herdr](https://github.com/herdrdev/herdr) that adds a command palette.

## What is different

`prefix+/` opens a palette listing every runnable action with its shortcut, filtered as
you type. Three of its commands move the focused pane to an existing space, a new space,
or a new tab, none of which had a keyboard path before. Moving a pane into an existing
tab prompts for which way it splits against that tab's focused pane. The palette also
lists any enabled plugin's actions and panes, so a plugin author's `herdr-plugin.toml`
commands are reachable by browsing instead of only through a bound key or the CLI/API.

This fork also carries a handful of smaller additions on top of upstream:

- Sanitized workspace/agent labels, so directory-derived names can't smuggle
  bidi overrides or other control characters into the sidebar.
- Improved Codex lifecycle detection and pane titles.
- An `agent type-submit` CLI/API primitive for driving an agent's input.
- Aligned trailing token groups in the sidebar.

The keybind reference on `?` behaves exactly as upstream ships it.

## Why it lives here

Upstream does not accept unsolicited implementation pull requests. Only maintainers and
accounts listed in its `.github/APPROVED_CONTRIBUTORS` allowlist may open one; everything
else is closed automatically, regardless of size, tests, or who wrote it. That is how
[herdrdev/herdr#2299](https://github.com/herdrdev/herdr/pull/2299) closed, under the
20-file / 1,000-line cap the policy used at the time. The idea is proposed in
[discussion #2283](https://github.com/herdrdev/herdr/discussions/2283); this fork exists
to keep using it in the meantime, and goes away if upstream takes it.

`master` tracks upstream and is rebuilt after each release.

## Install

macOS only.

```sh
brew tap cameronsjo/tap
brew uninstall herdr             # if the homebrew-core build is installed
brew install cameronsjo/tap/herdr
```

Back to upstream stable at any time:

```sh
brew uninstall cameronsjo/tap/herdr
brew install herdr
```

In a remote session the server owns the keybinding, so the remote machine needs this
build too.

## Everything else

Docs, quick start, and support are upstream at [herdr.dev](https://herdr.dev).
