# JLUCraft federation integration

Folly Launcher (formerly branded JLUCraft Launcher) — fork base:
[UNIkeEN/SJMCL](https://github.com/UNIkeEN/SJMCL), maintained on the
`folly` branch. Upstream launch/account/instance functions remain in place.
Federation navigation opens `/federation`.

## Architecture

The launcher is a thin client over the shared [`union-core`](../../union-core)
crate — the same infrastructure used by the Android manager
(`manager-app/federation-app`). Club-agnostic protocol logic (identity,
governance journal, game gateway) lives in union-core; this fork only adds:

- `src-tauri/src/union/` — Tauri commands: device identity, governance
  requests via the union-core client, and the loopback→union game proxy.
- `src/services/union.ts` — frontend bindings for those commands.
- `src/pages/federation.tsx`, `src/components/federation/` — league UI
  (teams, invites, registration, scoreboards, member governance).
- `src-tauri/src/launch/commands.rs` — launch hook: while a federation
  connection is active, the instance's launch-time auto-join target is
  overridden (saved instance settings are not overwritten).

Players connect to an authority, create teams, invite members, accept
invitations, submit rosters and read both scoreboards. Enrollment is
controlled by the authority administrator.

## Game connection

`<scheme>://connect?peer=…&service=…&address=…` links (scheme is brand-dependent,
see below) are handled in two ways:

- The OS-level deep link registered in `tauri.conf.json` reaches the
  federation page through `src/hooks/deep-link.ts` and auto-fills the
  connection link.
- The connection link can also be pasted manually.

Starting the proxy opens an owned loopback-to-union connection; disconnect
tears down the listener and sessions. Launching an instance while connected
keeps the launcher alive and points the game at the local address.

Links with `school=1` obtain a device-bound school credential from the
selected third-party account's saved HTTPS provider
(`<auth_server>/api/student-credential`), never a link-supplied token
destination. Credentials refresh every ten minutes; a failed refresh or a
changed selected profile closes the connection and the user must reconnect
after authenticating. The native game gateway performs Minecraft identity and
room routing checks. For headless debugging and smoke tests, the same proxy is
available as `unionctl proxy` in the union-core repository; everyday players
use the federation page in this launcher.

## Branding (multi-club)

The same union infrastructure serves multiple clubs. A club's identity is
data-driven, not code-driven:

1. Create `brands/<club>/brand.json`:

   ```json
   {
     "id": "myclub",
     "productName": "MyClub Launcher",
     "shortName": "MyClub",
     "statsPrefix": "myclub",
     "identifier": "org.myclub.launcher",
     "deeplinkScheme": "myclub",
     "copyright": "Copyright © 2026 MyClub",
     "federation": { "defaultAddress": "", "defaultPeer": "" }
   }
   ```

   - `identifier`: reverse-DNS bundle identifier; must be unique per club.
   - `deeplinkScheme`: lowercase URI scheme for club links.
   - `statsPrefix`: version prefix reported to the upstream statistics server
     per LICENSE.EXTRA (e.g. `folly` produces `folly-1.3.1`).
   - `federation.defaultAddress` / `defaultPeer`: optional pre-filled union
     connection for distributions handed out by organizers.
   - `shortName`: compact name for the navigation bar logo.

2. Optionally add `brands/<club>/icons/` (32x32.png, 64x64.png, 128x128.png,
   128x128@2x.png, icon.icns, icon.ico, icon.png); icons fall back to the
   upstream set when absent.
3. Run `pnpm brand <club>`. This rewrites the native identity in
   `src-tauri/tauri.conf.json` (product name, identifier, copyright, icons,
   deep-link scheme) and regenerates `src/brand.gen.json`, which the frontend
   reads at runtime via `src/brand.ts`.
4. Translate/reword the `FederationPage` and `MemberGovernance` sections (and
   `HeadNavBar.navList.federation`) in `src/locales/*.json` to taste.
5. Build and publish normally. Per upstream `LICENSE.EXTRA`, redistribution
   must not use the SJMCL name or logo — step 1–4 is what enforces that.

Upstream `main` stays unbranded: the committed default of
`src/brand.gen.json` keeps the SJMCL identity, so the brand script only needs
to run when building a club distribution.

## Validation status

Validated: TypeScript typecheck, ESLint on changed frontend files,
`cargo check`, and `pnpm locale diff en` (all eight locale files aligned).
The upstream Rust source emits pre-existing platform-specific warnings outside
the federation module.

**Not yet validated:** no graphical installation package, account login, or
live Minecraft match has been tested end-to-end.
