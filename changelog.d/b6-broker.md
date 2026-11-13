## Live documents

- textweaver's broker counts a login before checking its password, so a burst of wrong logins from one address gets no more answers than the five-failure lockout allows.
- The broker caps connections per address: 8 still logging in and 16 logged in. At most 50 readers may join with a join code, and a join reader can no longer push another off by reusing its client id.
- The failed-login limit keys IPv6 addresses by their /64 prefix, and keeps counting in a shared overflow bucket when its table is full, so a flood of addresses cannot switch it off.
- After 200 wrong join codes in one session, the join code turns off and the broker says so: "Join code off: too many wrong codes. Start the broker again for a new code." Accounts keep working.
- A plain listener on a 100.64.0.0/10 address is called Tailscale only when a Tailscale network interface is found (Linux); otherwise the broker warns that the link may not be encrypted, since carriers use the same range.
- On Linux and macOS, the broker's key and accounts file are set to owner-only after opening, so a stale file with wider permissions is narrowed.
- The join code documentation says what the code resists: its secret is one of about a million values, and the lockout, the caps and the session budget limit guessing.
