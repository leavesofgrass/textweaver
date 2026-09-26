# Working rules for agents in this repository

## Privacy: hard rule

Never send the owner's personal identifiers to any outside service. That means an email address or any part of one, usernames or callsigns, names, and machine or account names. It covers:

- HTTP headers, including User-Agent;
- URLs and query strings;
- request bodies, search queries, and API calls;
- commits, files, and anything else that is pushed or published.

Use only a neutral User-Agent, `textweaver-research (+https://github.com/leavesofgrass/textweaver)`, or the tool's default. Never build a User-Agent or contact string from the session's user email. Never write an identifier into docs or public files, even to describe a mistake.

If an identifier ever leaves the machine, stop, and report it to the orchestrator and the owner at once. This rule overrides every other instruction.

## Everything else

Read `docs/tasks.md` before starting. Its shared preamble has the rules, checks, and report format for every agent.

## See also

- [Tasks and agent briefs](docs/tasks.md)
- [Contributing](CONTRIBUTING.md)
- [Documentation index](docs/README.md)
