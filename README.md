<h1 align="center">
  :octocat: :key:
  <br/>
  octopat
</h1>

<p align="center">
   An interactive GitHub personal access token command line dispenser ✨
</p>

<div align="center">
  <a href="https://github.com/softprops/octopat/actions">
		<img src="https://github.com/softprops/octopat/workflows/Main/badge.svg"/>
	</a>
</div>

<br />

## Why

I often find myself needing to [generate personal access tokens](https://help.github.com/en/github/authenticating-to-github/creating-a-personal-access-token-for-the-command-line#using-a-token-on-the-command-line) for GitHub integrations and API access. I'm often working from the command line. Pausing to navigate though GitHub settings pages interrupts my flow.

Octopat is designed as a command line interface to work with my command line flow, not against it.

## Install

### Homebrew (on osx)

```sh
$ brew install softprops/tools/octopat
```

If you want to upgrade to newer versions, use `brew upgrade`. This will install the latest version.

```sh
$ brew upgrade softprops/tools/octopat
```

### Cargo install (rust users)

```sh
$ cargo install octopat
```

### GitHub Releases

You can download and install install precompiled binaries from a [GitHub Releases](https://github.com/softprops/octopat/releases) page.

You can programmatically install these using curl as well

```sh
$ cd $HOME/bin
$ curl -L "https://github.com/softprops/octopat/releases/download/v0.0.1/octopat-$(uname -s)-$(uname -m).tar.gz" \
  | tar -xz -C ~/bin
```

## How it works

In a nutshell, octopat is an embedded oauth application that copies access tokens to your clipboard.

1. When running octopat for the first time, you will be prompted for a set of GitHub app credentials. If you do not have a GitHub app go ahead an [create one here](https://developer.github.com/apps/building-oauth-apps/creating-an-oauth-app/). You will be asked for a for a few pieces of information when creating an app, a name and an Authorization URL.  
  
> It's name doesn't matter but you may want to use "octopat" for clarity.  
  
You will also be asked for Authorization callback URL. Set this to "http://localhost:4567/" which will be the url of the embedded octopat application running on your local host.  
  
> If you wish to use a different port, do so but provide it with the `-p` flag on the command line.  
  
Octopat will store these credentials securely on your local keychain so that you won't have to remember them on each run.

2. GitHub access tokens are scoped to specific capabilities. You'll be presented with a list of scopes to select from then be taken to GitHub to authorize access (to your own GitHub app).  

GitHub will then redirect your browser to a server embedded within the cli that will receive the authorization information and exchange it for an access token before copying it to your clipboard.

At no point is secret information stored insecurely or printed out.

## Revoking tokens

Since octopat is just an oauth application you can revoke tokens the [way you normally  would](https://help.github.com/en/github/authenticating-to-github/reviewing-your-authorized-applications-oauth)

There is a tradeoff with the oauth approach to generating tokens. You don't have an index for revoking one vs another. You can only revoke access given to a Github app. How do you overcome this tradeoff? You can make more than one GitHub app, one per category
of application you are building. `octopat` accepts an `--alias` flag so that you can target a specific app when provisioning new tokens.

## Why the oauth dance

This CLI uses the [web application oauth flow](https://developer.github.com/apps/building-oauth-apps/authorizing-oauth-apps/#web-application-flow) to dispense personal access tokens. Historically this has also been possible through a separate authorizations API which is [now deprecated](https://developer.github.com/changes/2020-02-14-deprecating-oauth-auth-endpoint/).

Doug Tangren (softprops) 2020