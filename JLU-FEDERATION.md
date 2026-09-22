# JLUCraft federation integration

Fork base: UNIkeEN/SJMCL. Upstream launch/account/instance functions remain in place. Federation navigation opens `/federation`.

The new `src/services/union.ts` calls `src-tauri/src/union` commands; native code stores a separate private device key in app_data_dir and calls adjacent union-core. Players can connect to a configured authority, create teams, invite members, accept invitations, submit rosters and read both scoreboards. Enrollment is controlled by the authority administrator.

The league page accepts jlucraft://connect links and starts/stops an owned loopback-to-union connection. Launching an instance while connected overrides its launch-time auto-join target and keeps the launcher alive; saved instance settings are not overwritten. Disconnect tears down the listener and sessions.

Links with school=1 obtain a device-bound school credential from the selected third-party account's saved HTTPS provider, never a link-supplied token destination. Credentials refresh every ten minutes. A failed refresh or changed selected profile closes the connection; the user must reconnect after authenticating. The native game gateway performs Minecraft identity and room routing checks. tools/union-launcher-cli remains available for standalone use and optional relay hosting.

Validation performed: TypeScript typecheck, lint of changed front-end files, Rust cargo check. The upstream Rust source emits existing platform-specific warnings outside the federation module. No graphical installation package, account login or live Minecraft match has been tested.
