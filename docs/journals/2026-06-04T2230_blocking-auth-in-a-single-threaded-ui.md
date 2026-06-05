---
created: 2026-06-04T22:30:00Z
branch: main
author: Claude Opus 4.8 (1M context)
status: active
sprint: STUDIO-2
---

# Technical Journal — June 4, 2026

## Threading a blocking OAuth flow into a single-threaded UI

### The problem

The native loopback-PKCE flow is, by nature, a *blocking* sequence: bind a
`127.0.0.1` port, open the browser, then **wait** for the user to come back
through the IdP — which could be two seconds or two minutes — then exchange the
code over the network. Slint, like most retained-mode UI toolkits, runs its
event loop on one thread and expects callbacks to return promptly. Call
`auth::login()` directly from the Sign-in button handler and the whole window
freezes on `server.incoming_requests()` until the callback lands. Unacceptable.

### The shape that worked

Three rules, and the flow drops into a single-threaded UI cleanly:

1. **The flow is a plain blocking function off the UI thread.** `auth::login()`
   knows nothing about Slint — it binds the listener, opens the browser, blocks
   on the callback, exchanges the code, validates, stores. The button handler
   spawns a `std::thread` and immediately returns, after flipping
   `auth-status = "signing"` so the chip shows progress.

2. **The result comes back through the event loop, not a channel.**
   `slint::invoke_from_event_loop(move || { ... })` marshals the
   `Result<AuthSession, AuthError>` back onto the UI thread to set
   `signed-in` / `wallet-address` / `auth-status`. No `Arc<Mutex<>>`, no manual
   channel drain — the closure *is* the message. The captured `Weak<StudioWindow>`
   and the `Result` are both `Send`, which is the only constraint to satisfy.

3. **Optimistic for sign-out, pessimistic for sign-in.** Sign-out clears the UI
   immediately and does the server revoke on a background thread (a failed revoke
   shouldn't keep you "signed in"). Sign-in waits for the real result before
   claiming success, because a wallet address you can't prove is worse than a
   spinner.

### The testing trick

The flow's logic — capture the callback, exchange the code, parse the response —
needs proof without a live IdP. Two in-process servers do it:

- Fire a raw `GET /auth/callback?code=…&state=…` at the loopback listener from a
  spawned thread and assert `wait_for_callback` returns the code and checks state.
- Stand up a `tiny_http` mock `/token` server that returns a canned token JSON,
  point `exchange_code` at it via `CITRATE_STUDIO_ISSUER`, and assert the
  `TokenSet` parses. This exercises the *actual* ureq POST + JSON path, not a
  mocked-out stub.

That's the difference between "the types line up" and "the flow runs." A raw
`TcpStream` write for the callback keeps the test from depending on a client lib
to test the server half.

### What bit

`tiny_http::Server::server_addr()` returns a `ListenAddr`, not a `SocketAddr` —
you go through `.to_ip()?.port()` to learn the ephemeral port you bound. Small,
but it's the port you have to bake into the `redirect_uri` *before* opening the
browser, so it's on the critical path.

### Lesson

A blocking flow and a single-threaded UI aren't in conflict — you just need a
worker thread to hold the block and `invoke_from_event_loop` to hand the result
back. Keep the flow toolkit-agnostic and the seam stays a four-line button
handler. And test the network half against an in-process server, not a mock of
your own code — the bugs live in the real POST.

---

*Journal entry by Claude (AI assistant), for Larry (human architect).*
*STUDIO-2 · loopback PKCE off-thread · in-process loopback + /token mocks.*
