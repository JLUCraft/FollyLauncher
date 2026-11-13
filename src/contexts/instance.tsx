import { open } from "@tauri-apps/plugin-dialog";
import { openPath } from "@tauri-apps/plugin-opener";
import { useRouter } from "next/router";
import React, {
  createContext,
  useCallback,
  useContext,
  useEffect,
} from "react";
import { useGlobalData, useGlobalDataDispatch } from "@/contexts/global-data";
import { useToast } from "@/contexts/toast";
import { InstanceSubdirType } from "@/enums/instance";
import { GetStateFlag, usePromisedGetState } from "@/hooks/get-state";
import { useKeyedState } from "@/hooks/keyed-state";
import { GameConfig } from "@/models/config";
import {
  InstanceSummary,
  LocalModInfo,
  ResourcePackInfo,
  SchematicInfo,
  ScreenshotInfo,
  ShaderPackInfo,
} from "@/models/instance/misc";
import { WorldInfo } from "@/models/instance/world";
import { InstanceService } from "@/services/instance";
import { updateByKeyPath } from "@/utils/partial";

type ImportResourcesOptions = {
  filterName: string;
  filterExt: string[];
  tgtDirType: InstanceSubdirType;
  paths?: string[];
  multiple?: boolean;
  decompress?: boolean;
  onSuccessCallback: () => void;
};

export interface InstanceContextType {
  instanceId: string | undefined;
  summary: InstanceSummary | undefined;
  updateSummaryInContext: (path: string, value: unknown) => void;
  gameConfig: GameConfig | undefined;
  openInstanceSubdir: (dirType: InstanceSubdirType) => void;
  // retrieve instance resource data with frontend cache
  getWorldList: (
    sync?: boolean
  ) => Promise<WorldInfo[] | GetStateFlag | undefined>;
  isWorldListLoading: boolean;
  getLocalModList: (
    sync?: boolean
  ) => Promise<LocalModInfo[] | GetStateFlag | undefined>;
  isLocalModListLoading: boolean;
  getResourcePackList: (
    sync?: boolean
  ) => Promise<ResourcePackInfo[] | GetStateFlag | undefined>;
  isResourcePackListLoading: boolean;
  getServerResourcePackList: (
    sync?: boolean
  ) => Promise<ResourcePackInfo[] | GetStateFlag | undefined>;
  isServerResourcePackListLoading: boolean;
  getSchematicList: (
    sync?: boolean
  ) => Promise<SchematicInfo[] | GetStateFlag | undefined>;
  isSchematicListLoading: boolean;
  getShaderPackList: (
    sync?: boolean
  ) => Promise<ShaderPackInfo[] | GetStateFlag | undefined>;
  isShaderPackListLoading: boolean;
  getScreenshotList: (
    sync?: boolean
  ) => Promise<ScreenshotInfo[] | GetStateFlag | undefined>;
  isScreenshotListLoading: boolean;
  // getInstanceGameConfig: (sync?: boolean) => GameConfig | undefined;
  // shared service handler
  handleRetrieveInstanceSubdirPath: (
    dirType: InstanceSubdirType
  ) => Promise<string | null>;
  handleImportResources: (options: ImportResourcesOptions) => void;
  handleUpdateInstanceConfig: (path: string, value: unknown) => void;
  handleResetInstanceGameConfig: () => void;
}

export const InstanceContext = createContext<InstanceContextType | undefined>(
  undefined
);

export const InstanceContextProvider: React.FC<{
  children: React.ReactNode;
}> = ({ children }) => {
  const router = useRouter();
  const toast = useToast();
  const { getInstanceList, instanceList } = useGlobalData();
  const { setInstanceList } = useGlobalDataDispatch();

  const instanceIdRaw = router.query.id;
  const instanceId = Array.isArray(instanceIdRaw)
    ? instanceIdRaw[0]
    : instanceIdRaw;

  const [instanceGameConfig, setInstanceGameConfig] = useKeyedState<
    GameConfig | undefined
  >(undefined, [instanceId]);

  const [worlds, setWorlds] = useKeyedState<WorldInfo[] | undefined>(
    undefined,
    [instanceId]
  );
  const [localMods, setLocalMods] = useKeyedState<LocalModInfo[] | undefined>(
    undefined,
    [instanceId]
  );
  const [resourcePacks, setResourcePacks] = useKeyedState<
    ResourcePackInfo[] | undefined
  >(undefined, [instanceId]);
  const [serverResourcePacks, setServerResourcePacks] = useKeyedState<
    ResourcePackInfo[] | undefined
  >(undefined, [instanceId]);
  const [schematics, setSchematics] = useKeyedState<
    SchematicInfo[] | undefined
  >(undefined, [instanceId]);
  const [shaderPacks, setShaderPacks] = useKeyedState<
    ShaderPackInfo[] | undefined
  >(undefined, [instanceId]);
  const [screenshots, setScreenshots] = useKeyedState<
    ScreenshotInfo[] | undefined
  >(undefined, [instanceId]);

  const instanceSummary = instanceList?.find(
    (instance) => instance.id === instanceId
  );

  const summaryIdRef = React.useRef<string | undefined>(undefined);
  React.useLayoutEffect(() => {
    summaryIdRef.current = instanceId;
  }, [instanceId]);

  const clearAllResState = useCallback(() => {
    setWorlds(undefined);
    setLocalMods(undefined);
    setResourcePacks(undefined);
    setServerResourcePacks(undefined);
    setSchematics(undefined);
    setShaderPacks(undefined);
    setScreenshots(undefined);
  }, [
    setLocalMods,
    setResourcePacks,
    setSchematics,
    setScreenshots,
    setServerResourcePacks,
    setShaderPacks,
    setWorlds,
  ]);

  const updateSummaryInContext = useCallback(
    (path: string, value: unknown) => {
      // for frontend-only state update to sync with backend if needed.
      if (!instanceSummary) return;
      const next = { ...instanceSummary };
      updateByKeyPath(next, path, value);
      setInstanceList(
        (getInstanceList() || []).map((instance) =>
          instance.id === next.id ? next : instance
        )
      );
    },
    [getInstanceList, setInstanceList, instanceSummary]
  );

  const handleRetrieveInstanceGameConfig = useCallback(
    (id: string) => {
      if (id !== undefined && id) {
        InstanceService.retrieveInstanceGameConfig(id)
          .then((response) => {
            if (response.status === "success") {
              setInstanceGameConfig(response.data);
            } else {
              toast({
                title: response.message,
                description: response.details,
                status: "error",
              });
            }
          })
          .catch((error: unknown) => console.error(error));
      }
    },
    [setInstanceGameConfig, toast]
  );

  useEffect(() => {
    getInstanceList();
    if (instanceId && instanceSummary?.id)
      handleRetrieveInstanceGameConfig(instanceId);
  }, [
    instanceId,
    instanceSummary?.id,
    getInstanceList,
    handleRetrieveInstanceGameConfig,
  ]);

  const handleRetrieveInstanceSubdirPath = useCallback(
    (dirType: InstanceSubdirType): Promise<string | null> => {
      if (instanceId !== undefined) {
        return InstanceService.retrieveInstanceSubdirPath(
          instanceId,
          dirType
        ).then((response) => {
          if (response.status === "success") {
            return response.data;
          } else {
            toast({
              title: response.message,
              description: response.details,
              status: "error",
            });
            return null;
          }
        });
      }
      return Promise.resolve(null);
    },
    [instanceId, toast]
  );

  const openInstanceSubdir = useCallback(
    async (dirType: InstanceSubdirType) => {
      const path = await handleRetrieveInstanceSubdirPath(dirType);
      if (path) await openPath(path);
    },
    [handleRetrieveInstanceSubdirPath]
  );

  const handleImportResources = useCallback(
    (options: ImportResourcesOptions) => {
      const {
        filterName,
        filterExt,
        tgtDirType,
        paths,
        multiple = false,
        decompress = false,
        onSuccessCallback,
      } = options;
      if (instanceId !== undefined) {
        const selectedPathPromise = paths
          ? Promise.resolve(paths)
          : open({
              multiple,
              filters: [
                {
                  name: filterName,
                  extensions: filterExt,
                },
              ],
            });

        selectedPathPromise
          .then((selectedPath) => {
            const selectedPaths = Array.isArray(selectedPath)
              ? selectedPath
              : selectedPath
                ? [selectedPath]
                : [];
            if (selectedPaths.length === 0) return;
            InstanceService.copyResourcesToInstances(
              selectedPaths,
              [instanceId],
              tgtDirType,
              decompress
            )
              .then((response) => {
                if (response.status === "success") {
                  toast({
                    title: response.message,
                    status: "success",
                  });
                  onSuccessCallback();
                  // KNOWN ISSUE: When the successfully copied file cannot be loaded as world/mod etc. But this handler will still toast success.
                } else
                  toast({
                    title: response.message,
                    description: response.details,
                    status: "error",
                  });
              })
              .catch((error: unknown) => console.error(error));
          })
          .catch((error: unknown) => console.error(error));
      }
    },
    [instanceId, toast]
  );

  const handleRetrieveWorldList = useCallback(async () => {
    if (summaryIdRef.current === undefined) {
      return;
    }
    let lastSummaryIdRef = summaryIdRef.current;
    const response = await InstanceService.retrieveWorldList(
      summaryIdRef.current
    );
    if (lastSummaryIdRef !== summaryIdRef.current) {
      return GetStateFlag.Cancelled;
    }
    if (response.status === "success") {
      const sorted = [...response.data].sort(
        (a, b) => b.lastPlayedAt - a.lastPlayedAt
      );
      setWorlds(sorted);
      return sorted;
    } else {
      toast({
        title: response.message,
        description: response.details,
        status: "error",
      });
      setWorlds([]);
      return [];
    }
  }, [setWorlds, toast]);

  const handleRetrieveLocalModList = useCallback(async () => {
    if (summaryIdRef.current === undefined) {
      return;
    }
    let lastSummaryIdRef = summaryIdRef.current;
    const response = await InstanceService.retrieveLocalModList(
      summaryIdRef.current
    );
    if (lastSummaryIdRef !== summaryIdRef.current) {
      return GetStateFlag.Cancelled;
    }
    if (response.status === "success") {
      setLocalMods(response.data);
      return response.data;
    } else {
      toast({
        title: response.message,
        description: response.details,
        status: "error",
      });
      setLocalMods([]);
      return [];
    }
  }, [setLocalMods, toast]);

  const handleRetrieveResourcePackList = useCallback(async () => {
    if (summaryIdRef.current === undefined) {
      return;
    }
    let lastSummaryIdRef = summaryIdRef.current;
    const response = await InstanceService.retrieveResourcePackList(
      summaryIdRef.current
    );
    if (lastSummaryIdRef !== summaryIdRef.current) {
      return GetStateFlag.Cancelled;
    }
    if (response.status === "success") {
      setResourcePacks(response.data);
      return response.data;
    } else {
      toast({
        title: response.message,
        description: response.details,
        status: "error",
      });
      setResourcePacks([]);
      return [];
    }
  }, [setResourcePacks, toast]);

  const handleServerRetrieveResourcePackList = useCallback(async () => {
    if (summaryIdRef.current === undefined) {
      return;
    }
    let lastSummaryIdRef = summaryIdRef.current;
    const response = await InstanceService.retrieveServerResourcePackList(
      summaryIdRef.current
    );
    if (lastSummaryIdRef !== summaryIdRef.current) {
      return GetStateFlag.Cancelled;
    }
    if (response.status === "success") {
      setServerResourcePacks(response.data);
      return response.data;
    } else {
      toast({
        title: response.message,
        description: response.details,
        status: "error",
      });
      setServerResourcePacks([]);
      return [];
    }
  }, [setServerResourcePacks, toast]);

  const handleRetrieveSchematicList = useCallback(async () => {
    if (summaryIdRef.current === undefined) {
      return;
    }
    let lastSummaryIdRef = summaryIdRef.current;
    const response = await InstanceService.retrieveSchematicList(
      summaryIdRef.current
    );
    if (lastSummaryIdRef !== summaryIdRef.current) {
      return GetStateFlag.Cancelled;
    }
    if (response.status === "success") {
      setSchematics(response.data);
      return response.data;
    } else {
      toast({
        title: response.message,
        description: response.details,
        status: "error",
      });
      setSchematics([]);
      return [];
    }
  }, [setSchematics, toast]);

  const handleRetrieveShaderPackList = useCallback(async () => {
    if (summaryIdRef.current === undefined) {
      return;
    }
    let lastSummaryIdRef = summaryIdRef.current;
    const response = await InstanceService.retrieveShaderPackList(
      summaryIdRef.current
    );
    if (lastSummaryIdRef !== summaryIdRef.current) {
      return GetStateFlag.Cancelled;
    }
    if (response.status === "success") {
      setShaderPacks(response.data);
      return response.data;
    } else {
      toast({
        title: response.message,
        description: response.details,
        status: "error",
      });
      setShaderPacks([]);
      return [];
    }
  }, [setShaderPacks, toast]);

  const handleRetrieveScreenshotList = useCallback(async () => {
    if (summaryIdRef.current === undefined) {
      return;
    }
    let lastSummaryIdRef = summaryIdRef.current;
    const response = await InstanceService.retrieveScreenshotList(
      summaryIdRef.current
    );
    if (lastSummaryIdRef !== summaryIdRef.current) {
      return GetStateFlag.Cancelled;
    }
    if (response.status === "success") {
      const sorted = [...response.data].sort((a, b) => b.time - a.time);
      setScreenshots(sorted);
      return sorted;
    } else {
      toast({
        title: response.message,
        description: response.details,
        status: "error",
      });
      setScreenshots([]);
      return [];
    }
  }, [setScreenshots, toast]);

  const handleUpdateInstanceConfig = useCallback(
    (path: string, value: unknown) => {
      if (instanceId !== undefined) {
        InstanceService.updateInstanceConfig(instanceId, path, value)
          .then((response) => {
            if (response.status !== "success") {
              toast({
                title: response.message,
                description: response.details,
                status: "error",
              });
            } else {
              if (path.startsWith("specGameConfig")) {
                const newConfig = { ...instanceGameConfig };
                updateByKeyPath(
                  newConfig,
                  path.replace("specGameConfig.", ""),
                  value
                );
                setInstanceGameConfig(newConfig as GameConfig);
                // version isolation is shared by summary and game config struct.
                if (path === "specGameConfig.versionIsolation")
                  updateSummaryInContext("isVersionIsolated", value);
                clearAllResState();
              } else if (path === "useSpecGameConfig") {
                updateSummaryInContext(path, value);
                if (value) handleRetrieveInstanceGameConfig(instanceId);
                // clear all cached resource state due to version isolation may change.
                clearAllResState();
              } else {
                updateSummaryInContext(path, value);
              }
            }
          })
          .catch((error: unknown) => console.error(error));
      }
    },
    [
      instanceId,
      instanceGameConfig,
      handleRetrieveInstanceGameConfig,
      setInstanceGameConfig,
      toast,
      updateSummaryInContext,
      clearAllResState,
    ]
  );

  const handleResetInstanceGameConfig = useCallback(() => {
    if (instanceId !== undefined) {
      InstanceService.resetInstanceGameConfig(instanceId)
        .then((response) => {
          if (response.status === "success") {
            toast({
              title: response.message,
              status: "success",
            });
            handleRetrieveInstanceGameConfig(instanceId);
          } else
            toast({
              title: response.message,
              description: response.details,
              status: "error",
            });
        })
        .catch((error: unknown) => console.error(error));
    }
  }, [instanceId, handleRetrieveInstanceGameConfig, toast]);

  const [getWorldList, isWorldListLoading] = usePromisedGetState(
    worlds,
    instanceId,
    handleRetrieveWorldList
  );

  const [getLocalModList, isLocalModListLoading] = usePromisedGetState(
    localMods,
    instanceId,
    handleRetrieveLocalModList
  );

  const [getResourcePackList, isResourcePackListLoading] = usePromisedGetState(
    resourcePacks,
    instanceId,
    handleRetrieveResourcePackList
  );

  const [getServerResourcePackList, isServerResourcePackListLoading] =
    usePromisedGetState(
      serverResourcePacks,
      instanceId,
      handleServerRetrieveResourcePackList
    );

  const [getSchematicList, isSchematicListLoading] = usePromisedGetState(
    schematics,
    instanceId,
    handleRetrieveSchematicList
  );

  const [getShaderPackList, isShaderPackListLoading] = usePromisedGetState(
    shaderPacks,
    instanceId,
    handleRetrieveShaderPackList
  );

  const [getScreenshotList, isScreenshotListLoading] = usePromisedGetState(
    screenshots,
    instanceId,
    handleRetrieveScreenshotList
  );

  useEffect(() => {
    if (instanceId) {
      getLocalModList()
        .then((mods) => {
          if (mods === GetStateFlag.Cancelled) return;
          setLocalMods(mods);
        })
        .catch((error: unknown) => console.error(error));
      getWorldList()
        .then((worlds) => {
          if (worlds === GetStateFlag.Cancelled) return;
          setWorlds(worlds);
        })
        .catch((error: unknown) => console.error(error));
      getResourcePackList()
        .then((packs) => {
          if (packs === GetStateFlag.Cancelled) return;
          setResourcePacks(packs);
        })
        .catch((error: unknown) => console.error(error));
      getServerResourcePackList()
        .then((packs) => {
          if (packs === GetStateFlag.Cancelled) return;
          setServerResourcePacks(packs);
        })
        .catch((error: unknown) => console.error(error));
      getSchematicList()
        .then((schematics) => {
          if (schematics === GetStateFlag.Cancelled) return;
          setSchematics(schematics);
        })
        .catch((error: unknown) => console.error(error));
      getShaderPackList()
        .then((packs) => {
          if (packs === GetStateFlag.Cancelled) return;
          setShaderPacks(packs);
        })
        .catch((error: unknown) => console.error(error));
      getScreenshotList()
        .then((screenshots) => {
          if (screenshots === GetStateFlag.Cancelled) return;
          setScreenshots(screenshots);
        })
        .catch((error: unknown) => console.error(error));
    }
  }, [
    getLocalModList,
    getResourcePackList,
    getSchematicList,
    getScreenshotList,
    getServerResourcePackList,
    getShaderPackList,
    getWorldList,
    instanceId,
    setLocalMods,
    setResourcePacks,
    setSchematics,
    setScreenshots,
    setServerResourcePacks,
    setShaderPacks,
    setWorlds,
  ]);

  // const getInstanceGameConfig = useGetState(
  //   instanceGameConfig,
  //   handleRetrieveInstanceGameConfig
  // );

  return (
    <InstanceContext.Provider
      value={{
        instanceId: instanceId,
        summary: instanceSummary,
        updateSummaryInContext,
        gameConfig: instanceGameConfig,
        openInstanceSubdir,
        getWorldList,
        isWorldListLoading,
        getLocalModList,
        isLocalModListLoading,
        getResourcePackList,
        isResourcePackListLoading,
        getServerResourcePackList,
        isServerResourcePackListLoading,
        getSchematicList,
        isSchematicListLoading,
        getShaderPackList,
        isShaderPackListLoading,
        getScreenshotList,
        isScreenshotListLoading,
        // getInstanceGameConfig,
        handleRetrieveInstanceSubdirPath,
        handleImportResources,
        handleUpdateInstanceConfig,
        handleResetInstanceGameConfig,
      }}
    >
      {children}
    </InstanceContext.Provider>
  );
};

export const useInstanceSharedData = (): InstanceContextType => {
  const context = useContext(InstanceContext);
  if (!context) {
    throw new Error(
      "useInstanceSharedData must be used within a InstanceContextProvider"
    );
  }
  return context;
};
