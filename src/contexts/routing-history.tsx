import { useRouter } from "next/router";
import React, { createContext, useContext, useEffect, useState } from "react";

type RoutingHistoryContextType = {
  history: string[];
  removeHistory: (prefix: string) => void;
  replaceHistory: (src: string, tgt: string) => void;
};

const MAX_ROUTING_HISTORY = 200;

const RoutingHistoryContext = createContext<
  RoutingHistoryContextType | undefined
>(undefined);

export const RoutingHistoryContextProvider: React.FC<{
  children: React.ReactNode;
}> = ({ children }) => {
  const router = useRouter();
  const [history, setHistory] = useState<string[]>([]);
  const [recordedPath, setRecordedPath] = useState<string>();

  if (router.isReady && recordedPath !== router.asPath) {
    setRecordedPath(router.asPath);
    setHistory((previous) =>
      previous[previous.length - 1] === router.asPath
        ? previous
        : [...previous.slice(-(MAX_ROUTING_HISTORY - 1)), router.asPath]
    );
  }

  useEffect(() => {
    if (!router.isReady) return;
    if (window.logger)
      logger
        .info("Frontend navigated to:", router.asPath)
        .catch((error: unknown) => console.error(error));
  }, [router.isReady, router.asPath]);

  const removeHistory = (prefix: string) => {
    setHistory((prev) => prev.filter((route) => !route.startsWith(prefix)));
  };

  const replaceHistory = (src: string, tgt: string) => {
    setHistory((prev) => prev.map((route) => route.replaceAll(src, tgt)));
  };

  return (
    <RoutingHistoryContext.Provider
      value={{ history, removeHistory, replaceHistory }}
    >
      {children}
    </RoutingHistoryContext.Provider>
  );
};

export const useRoutingHistory = (): RoutingHistoryContextType => {
  const context = useContext(RoutingHistoryContext);
  if (!context) {
    throw new Error(
      "useRoutingHistory must be used within a RoutingHistoryContextProvider"
    );
  }
  return context;
};
