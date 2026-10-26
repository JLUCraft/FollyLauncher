const assert = require("node:assert/strict");
const { readFileSync } = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");
const ts = require("typescript");

function deferred() {
  let resolve;
  const promise = new Promise((done) => (resolve = done));
  return { promise, resolve };
}

function fixture() {
  const slots = [];
  let cursor = 0;
  const discovery = deferred();
  const identity = deferred();
  const identityStarted = deferred();
  const calls = { identity: [], persistedIdentity: [], requests: [] };
  const storage = new Map();
  const react = {
    createContext: () => ({}),
    useCallback: (callback) => callback,
    useEffect: () => {},
    useRef(value) {
      const index = cursor++;
      return (slots[index] ||= { current: value });
    },
    useState(value) {
      const index = cursor++;
      if (!(index in slots)) slots[index] = value;
      return [
        slots[index],
        (next) => {
          slots[index] = typeof next === "function" ? next(slots[index]) : next;
        },
      ];
    },
  };
  const modules = {
    react,
    "react/jsx-runtime": {},
    "next/router": { useRouter: () => ({ push: () => {} }) },
    "react-i18next": {},
    "@/brand": { brand: { federation: { genesisHash: "pin" } } },
    "@/hooks/deep-link": { useDeepLink: () => {} },
    "@/services/union": {
      UnionService: {
        discover: () => discovery.promise,
        identityConfiguration: async (value) => {
          calls.identity.push(value);
          identityStarted.resolve();
          await identity.promise;
          // A dispatched native command can save before its reply reaches the hook.
          calls.persistedIdentity.push(value);
        },
        request: async (...args) => {
          calls.requests.push(args);
          return { revision: 1, members: {} };
        },
      },
    },
    "@/utils/federation/connection-link": {
      GENESIS_PATTERN: /^[a-f0-9]{64}$/,
      parseConnectionLink: (link) => (link ? JSON.parse(link) : null),
    },
    "@/utils/federation/errors": {},
    "@/utils/federation/navigation": {},
  };
  const source =
    readFileSync(
      path.join(__dirname, "../../src/contexts/federation.tsx"),
      "utf8"
    ) + "\nexport { useFederationState };";
  const compiled = ts.transpileModule(source, {
    compilerOptions: {
      module: ts.ModuleKind.CommonJS,
      jsx: ts.JsxEmit.ReactJSX,
    },
  }).outputText;
  const context = {
    exports: {},
    require: (name) => {
      assert.ok(name in modules, `Unexpected import: ${name}`);
      return modules[name];
    },
    localStorage: {
      setItem: (key, value) => storage.set(key, value),
      removeItem: (key) => storage.delete(key),
    },
  };
  vm.runInNewContext(compiled, context);
  return {
    calls,
    discovery,
    identity,
    identityStarted,
    storage,
    render: () => {
      cursor = 0;
      return context.exports.useFederationState();
    },
  };
}

const discoveredServer = {
  address: "discovered-address",
  peer: "discovered-peer",
  genesis: "identity-genesis",
  genesis_hash: "identity-hash",
  application: "a".repeat(64),
};
const chosenTarget = {
  address: "chosen-address",
  peer: "chosen-peer",
  genesis: "b".repeat(64),
};

(async () => {
  for (const stage of ["discovery", "identity"]) {
    for (const action of ["connect", "disconnect"]) {
      const test = fixture();
      const pending = test.render().discoverAndConnect();
      if (stage === "identity") {
        test.discovery.resolve([discoveredServer]);
        await test.identityStarted.promise;
        assert.equal(test.calls.identity.length, 1);
      }
      if (action === "connect") {
        test.render().connectWithLink(JSON.stringify(chosenTarget));
      } else {
        test.render().disconnect();
      }
      test.discovery.resolve([discoveredServer]);
      test.identity.resolve();
      await pending;
      const state = test.render();
      assert.equal(test.calls.requests.length, action === "connect" ? 1 : 0);
      assert.equal(test.calls.identity.length, stage === "identity" ? 1 : 0);
      assert.equal(
        test.calls.persistedIdentity.length,
        stage === "identity" ? 1 : 0
      );
      assert.equal(state.discovering, false);
      assert.equal(state.identityConfigured, null);
      if (action === "connect") {
        assert.equal(state.address, chosenTarget.address);
        assert.equal(state.activeTarget.peer, chosenTarget.peer);
        assert.equal(
          JSON.parse(test.storage.get("federation-connection")).peer,
          chosenTarget.peer
        );
      } else {
        assert.equal(state.activeTarget, undefined);
        assert.equal(test.storage.size, 0);
      }
      console.log(`PASS: stale ${stage} after ${action}.`);
    }
  }
  const test = fixture();
  const first = test.render().discoverAndConnect();
  const second = test.render().discoverAndConnect();
  test.discovery.resolve([discoveredServer]);
  test.identity.resolve();
  await Promise.all([first, second]);
  assert.equal(test.calls.identity.length, 1);
  assert.equal(test.calls.requests.length, 1);
  assert.equal(test.render().activeTarget.peer, discoveredServer.peer);
  assert.equal(test.render().discovering, false);
  console.log("PASS: only the newest discovery connects.");
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
