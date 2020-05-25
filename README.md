<h1 align="center">
  :octocat: :key:
  <br/>
  octopat
</h1>

<p align="center">
   GitHub personal access token dispenser
</p>

<div align="center">
  <a href="https://github.com/softprops/octopat/actions">
		<img src="https://github.com/softprops/octopat/workflows/Main/badge.svg"/>
	</a>
</div>

<br />

This cli uses the [web application oauth flow](https://developer.github.com/apps/building-oauth-apps/authorizing-oauth-apps/#web-application-flow) to dispense personal access tokens. Historically this has also been possible through a separate authorizations api which is [now deprecated](https://developer.github.com/changes/2020-02-14-deprecating-oauth-auth-endpoint/)

## How it works

When running octopat for the first time you will be prompted for a GitHub app credentials. It will store
them securely on your local keychain so that you won't have to remember them on each run.

GitHub access tokens are scoped to specific capabilities. You'll be presented with a list of scopes to select from
then be taken to GitHub to authorize access (to your own GitHub app). GitHub will then redirect you to a server embedded within the cli that will receive the authorization information and exchange it for an access token before copying it to
your clipboard.

At no point is secret information stored insecurely or printed out.

Doug Tangren (softprops) 2019