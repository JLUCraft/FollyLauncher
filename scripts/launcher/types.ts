import type { AccountService } from "@/services/account";
import type { nativeCommands } from "@/services/commands";
import type { ConfigService } from "@/services/config";
import type { ExtensionService } from "@/services/extension";
import type { InstanceService } from "@/services/instance";
import type { LaunchService } from "@/services/launch";
import type { ResourceService } from "@/services/resource";
import type { TaskService } from "@/services/task";
import type { UtilsService } from "@/services/utils";

type RawResult<Value> = Value extends { status: "success"; data: infer Data }
  ? Data
  : Value extends { status: "error" }
    ? never
    : Value;

type ServiceResults<Service, Bindings> = {
  [
    Method in keyof Bindings as Bindings[Method] & string
  ]: Method extends keyof Service
    ? Service[Method] extends (...args: never[]) => infer Result
      ? RawResult<Awaited<Result>>
      : never
    : never;
};

export type NativeResults = ServiceResults<
  typeof AccountService,
  typeof nativeCommands.account
> &
  ServiceResults<typeof ConfigService, typeof nativeCommands.config> &
  ServiceResults<typeof ExtensionService, typeof nativeCommands.extension> &
  ServiceResults<typeof InstanceService, typeof nativeCommands.instance> &
  ServiceResults<typeof LaunchService, typeof nativeCommands.launch> &
  ServiceResults<typeof ResourceService, typeof nativeCommands.resource> &
  ServiceResults<typeof TaskService, typeof nativeCommands.task> &
  ServiceResults<typeof UtilsService, typeof nativeCommands.utils>;

export interface NativeInvoke {
  <Command extends keyof NativeResults>(
    command: Command,
    args?: object,
    timeout?: number
  ): Promise<NativeResults[Command]>;
  (command: string, args?: object, timeout?: number): Promise<unknown>;
}

export interface CheckedNativeInvoke {
  <Command extends keyof NativeResults>(
    command: Command,
    args?: object,
    expectError?: false
  ): Promise<NativeResults[Command]>;
  (command: string, args: object, expectError: true): Promise<string>;
  (command: string, args?: object, expectError?: false): Promise<unknown>;
}

export type Evaluate = (
  expression: string,
  target?: string
) => Promise<unknown>;

export interface Target {
  targetId: string;
  protocolTargetId: string;
  url?: string;
  title?: string;
  type?: string;
}

export interface ShotOptions {
  allowDialogs?: boolean;
  nativeWindow?: string;
}

export interface CaptureContext {
  evaluate: Evaluate;
  delay: (milliseconds: number) => Promise<void>;
  shot: (name: string, step: string, options?: ShotOptions) => Promise<void>;
  temp: string;
  out: string;
  repo: string;
}

export interface DeviceFixture {
  stationDirectory: string;
  browserBin: string;
  browserRequest: string;
  station: { origin: string; email: string; password: string };
  cleanup: () => Promise<void>;
}

export interface NativeFixture extends Partial<DeviceFixture> {
  runtime: string;
  versionPath: string;
  profile: string;
  extensionZip: string;
  instanceId: string;
}

export interface FollowupContext extends CaptureContext {
  fixture: NativeFixture;
  windowId: string;
  appPid: number;
  targets: () => Target[];
  refreshTargets: () => Promise<Target[]>;
}

export interface ScenarioContext {
  temp: string;
  fixture: NativeFixture;
  invoke: NativeInvoke;
  route: (path: string) => Promise<void>;
  shot: CaptureContext["shot"];
  delay: CaptureContext["delay"];
}

export interface DeviceContext extends Omit<ScenarioContext, "temp"> {
  evaluate: Evaluate;
  until: (
    expression: string,
    timeout?: number,
    page?: string
  ) => Promise<unknown>;
  targets: FollowupContext["targets"];
  refreshTargets: FollowupContext["refreshTargets"];
  findChildWindow: (pid?: number) => string | undefined;
  clickElement: (expression: string) => Promise<void>;
}

export interface NativeCheck {
  name: string;
  startedAt: string;
  status?: "passed" | "failed";
  detail?: unknown;
  error?: string;
  body?: unknown;
  sourceHashes?: Record<string, string>;
  nativeProcessPid?: number;
  nativeBinarySha256?: string;
}

export interface NativeReport {
  startedAt: string;
  profileScope: string;
  reused: string[];
  checks: NativeCheck[];
  sources: Record<string, string>;
  profilePath?: string;
  history?: NativeCheck[];
  policy?: unknown;
  gameStartup?: {
    instanceId: string;
    stage: string;
    scope: string;
    launchingId?: number;
    javaPid?: number;
    logTail?: string[];
  };
  cleanroom?: {
    stage: string;
    runtime: string;
    java: string;
    selectedLoader?: import("@/models/resource").ModLoaderResourceInfo;
  };
  finishedAt?: string;
  status?: "passed" | "failed" | "incomplete";
}

export interface NativeManifest {
  libraries: {
    name: string;
    natives?: Record<string, string>;
    downloads?: { classifiers?: Record<string, { sha1: string }> };
  }[];
}
