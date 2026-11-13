import { Box, VStack } from "@chakra-ui/react";
import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
} from "react";
import HomeWidget from "@/components/extension/home-widget";
import { useLauncherConfig } from "@/contexts/config";
import { useExtensionHost } from "@/contexts/extension/host";
import { useKeyedState } from "@/hooks/keyed-state";
import {
  ExtensionHomeWidgetContribution,
  type HomeWidgetStateTuple,
} from "@/models/extension";
import { areArraysEqual, clamp } from "@/utils/math";

interface HomeWidgetContainerProps {
  maxWidth: number;
}

interface WidthBounds {
  lower: number;
  upper: number;
}

type PersistHomeWidgetState = (
  nextOrder?: string[],
  nextWidthMap?: Record<string, number>,
  nextCollapsedMap?: Record<string, boolean>
) => void;

const DEFAULT_WIDGET_WIDTH = 300;

const areHomeWidgetStatesEqual = (
  left: HomeWidgetStateTuple[],
  right: HomeWidgetStateTuple[]
) => {
  return areArraysEqual(
    left,
    right,
    (
      [leftIdentifier, leftWidth, leftCollapsed],
      [rightIdentifier, rightWidth, rightCollapsed]
    ) =>
      leftIdentifier === rightIdentifier &&
      leftWidth === rightWidth &&
      leftCollapsed === rightCollapsed
  );
};

const HomeWidgetContainer = ({ maxWidth }: HomeWidgetContainerProps) => {
  const { config, update } = useLauncherConfig();
  const { homeWidgets } = useExtensionHost();
  const persistedState = config.extension.homeWidgetState;

  const widgetsByIdentifier = useMemo(() => {
    return new Map(homeWidgets.map((widget) => [widget.identifier, widget]));
  }, [homeWidgets]);

  const visibleWidgetIdentifiers = useMemo(
    () => homeWidgets.map((widget) => widget.identifier),
    [homeWidgets]
  );

  const persistedStateMap = useMemo(() => {
    return new Map(
      persistedState.map(([identifier, width, collapsed]) => [
        identifier,
        { width, collapsed },
      ])
    );
  }, [persistedState]);

  const isHydrated = !config.mocked && visibleWidgetIdentifiers.length > 0;
  const widgetSnapshot = useMemo(() => {
    if (!isHydrated) return { order: [], widths: {}, collapsed: {} };
    const visibleWidgetSet = new Set(visibleWidgetIdentifiers);
    const nextOrder: string[] = [];
    const seenIdentifiers = new Set<string>();

    for (const [identifier] of persistedState) {
      if (
        visibleWidgetSet.has(identifier) &&
        !seenIdentifiers.has(identifier)
      ) {
        nextOrder.push(identifier);
        seenIdentifiers.add(identifier);
      }
    }

    for (const identifier of visibleWidgetIdentifiers) {
      if (!seenIdentifiers.has(identifier)) {
        nextOrder.push(identifier);
        seenIdentifiers.add(identifier);
      }
    }

    const nextWidthMap = Object.fromEntries<number>(
      homeWidgets.map((widget) => [
        widget.identifier,
        persistedStateMap.get(widget.identifier)?.width ??
          widget.defaultWidth ??
          DEFAULT_WIDGET_WIDTH,
      ])
    );
    const nextCollapsedMap = Object.fromEntries<boolean>(
      homeWidgets.map((widget) => [
        widget.identifier,
        persistedStateMap.get(widget.identifier)?.collapsed ?? false,
      ])
    );

    return {
      order: nextOrder,
      widths: nextWidthMap,
      collapsed: nextCollapsedMap,
    };
  }, [
    isHydrated,
    homeWidgets,
    persistedState,
    persistedStateMap,
    visibleWidgetIdentifiers,
  ]);
  const [widgetOrder, setWidgetOrder] = useKeyedState<string[]>(
    widgetSnapshot.order,
    [widgetSnapshot]
  );
  const [widgetWidthMap, setWidgetWidthMap] = useKeyedState<
    Record<string, number>
  >(widgetSnapshot.widths, [widgetSnapshot]);
  const [widgetCollapsedMap, setWidgetCollapsedMap] = useKeyedState<
    Record<string, boolean>
  >(widgetSnapshot.collapsed, [widgetSnapshot]);

  const widgetOrderRef = useRef<string[]>([]);
  const widgetWidthMapRef = useRef<Record<string, number>>({});
  const widgetCollapsedMapRef = useRef<Record<string, boolean>>({});

  const persistHomeWidgetStateRef = useRef<PersistHomeWidgetState>(
    () => undefined
  );

  useLayoutEffect(() => {
    widgetOrderRef.current = widgetOrder;
  }, [widgetOrder]);

  useLayoutEffect(() => {
    widgetWidthMapRef.current = widgetWidthMap;
  }, [widgetWidthMap]);

  useLayoutEffect(() => {
    widgetCollapsedMapRef.current = widgetCollapsedMap;
  }, [widgetCollapsedMap]);

  useEffect(() => {
    if (!isHydrated) return;
    persistHomeWidgetStateRef.current(
      widgetSnapshot.order,
      widgetSnapshot.widths,
      widgetSnapshot.collapsed
    );
  }, [isHydrated, widgetSnapshot]);

  const getWidgetWidthBounds = useCallback(
    (widget: ExtensionHomeWidgetContribution): WidthBounds => {
      const lower = Math.max(0, widget.minWidth ?? 0);
      const upper = Math.max(
        lower,
        Math.min(maxWidth, widget.maxWidth ?? Number.POSITIVE_INFINITY)
      );
      return { lower, upper };
    },
    [maxWidth]
  );

  const orderedWidgetIdentifiers = useMemo(() => {
    const orderedIdentifiers = widgetOrder.filter((identifier) =>
      widgetsByIdentifier.has(identifier)
    );
    const missingIdentifiers = visibleWidgetIdentifiers.filter(
      (identifier) => !orderedIdentifiers.includes(identifier)
    );

    return [...orderedIdentifiers, ...missingIdentifiers];
  }, [visibleWidgetIdentifiers, widgetOrder, widgetsByIdentifier]);

  // Merge current UI state with hidden widgets so launcher config can fully restore the layout.
  const persistHomeWidgetState = useCallback(
    (
      nextOrder = widgetOrderRef.current,
      nextWidthMap = widgetWidthMapRef.current,
      nextCollapsedMap = widgetCollapsedMapRef.current
    ) => {
      const normalizedOrder = [
        ...nextOrder.filter((identifier) =>
          widgetsByIdentifier.has(identifier)
        ),
        ...visibleWidgetIdentifiers.filter(
          (identifier) => !nextOrder.includes(identifier)
        ),
      ];

      const nextVisibleState = normalizedOrder
        .map((identifier) => widgetsByIdentifier.get(identifier))
        .filter((widget) => !!widget)
        .map((widget) => {
          const bounds = getWidgetWidthBounds(widget);
          const preferredWidth =
            nextWidthMap[widget.identifier] ??
            widget.defaultWidth ??
            DEFAULT_WIDGET_WIDTH;
          const width = Math.round(
            clamp(preferredWidth, bounds.lower, bounds.upper)
          );

          return [
            widget.identifier,
            width,
            !!nextCollapsedMap[widget.identifier],
          ] as HomeWidgetStateTuple;
        });

      const nextVisibleStateMap = new Map(
        nextVisibleState.map((state) => [state[0], state])
      );
      const persistedIdentifierSet = new Set(
        persistedState.map(([identifier]) => identifier)
      );

      // Keep hidden widgets in their original persisted slots so partial startup
      // registration does not rewrite the whole order
      const reorderedVisibleState = [...nextVisibleState];
      const nextState = persistedState.map((state) => {
        const identifier = state[0];
        if (!nextVisibleStateMap.has(identifier)) return state;
        return reorderedVisibleState.shift() || state;
      });

      nextState.push(
        ...nextVisibleState.filter(
          ([identifier]) => !persistedIdentifierSet.has(identifier)
        )
      );

      if (!areHomeWidgetStatesEqual(persistedState, nextState)) {
        update("extension.homeWidgetState", nextState);
      }
    },
    [
      getWidgetWidthBounds,
      persistedState,
      update,
      visibleWidgetIdentifiers,
      widgetsByIdentifier,
    ]
  );

  useLayoutEffect(() => {
    persistHomeWidgetStateRef.current = persistHomeWidgetState;
  }, [persistHomeWidgetState]);

  const widgetLayouts = useMemo(() => {
    return orderedWidgetIdentifiers
      .map((identifier) => widgetsByIdentifier.get(identifier))
      .filter((widget) => !!widget)
      .map((widget) => {
        const bounds = getWidgetWidthBounds(widget);
        const preferredWidth =
          widgetWidthMap[widget.identifier] ??
          widget.defaultWidth ??
          DEFAULT_WIDGET_WIDTH;
        const width = clamp(preferredWidth, bounds.lower, bounds.upper);

        return {
          widget,
          bounds,
          width,
        };
      });
  }, [
    getWidgetWidthBounds,
    orderedWidgetIdentifiers,
    widgetWidthMap,
    widgetsByIdentifier,
  ]);

  const handleWidgetWidthChange = useCallback(
    (widgetIdentifier: string, width: number) => {
      setWidgetWidthMap((prev) => {
        if (prev[widgetIdentifier] === width) return prev;

        return {
          ...prev,
          [widgetIdentifier]: width,
        };
      });
    },
    [setWidgetWidthMap]
  );

  // Commit the final width on mouse up so drag updates don't spam config writes.
  const handleWidgetWidthCommit = useCallback(
    (widgetIdentifier: string, width: number) => {
      const nextWidthMap = {
        ...widgetWidthMapRef.current,
        [widgetIdentifier]: width,
      };
      widgetWidthMapRef.current = nextWidthMap;
      setWidgetWidthMap(nextWidthMap);
      persistHomeWidgetState(undefined, nextWidthMap);
    },
    [persistHomeWidgetState, setWidgetWidthMap]
  );

  // Persist collapsed state immediately because this action is discrete and cheap to save.
  const handleWidgetCollapsedChange = useCallback(
    (widgetIdentifier: string, collapsed: boolean) => {
      const nextCollapsedMap = {
        ...widgetCollapsedMapRef.current,
        [widgetIdentifier]: collapsed,
      };
      widgetCollapsedMapRef.current = nextCollapsedMap;
      setWidgetCollapsedMap(nextCollapsedMap);
      persistHomeWidgetState(undefined, undefined, nextCollapsedMap);
    },
    [persistHomeWidgetState, setWidgetCollapsedMap]
  );

  const handleMoveWidget = useCallback(
    (widgetIdentifier: string, direction: -1 | 1) => {
      const currentOrder = [...orderedWidgetIdentifiers];
      const currentIndex = currentOrder.indexOf(widgetIdentifier);
      const nextIndex = currentIndex + direction;

      if (
        currentIndex < 0 ||
        nextIndex < 0 ||
        nextIndex >= currentOrder.length
      ) {
        return;
      }

      [currentOrder[currentIndex], currentOrder[nextIndex]] = [
        currentOrder[nextIndex],
        currentOrder[currentIndex],
      ];

      widgetOrderRef.current = currentOrder;
      setWidgetOrder(currentOrder);
      persistHomeWidgetState(currentOrder);
    },
    [orderedWidgetIdentifiers, persistHomeWidgetState, setWidgetOrder]
  );

  const containerWidth = useMemo(
    () => Math.max(0, ...widgetLayouts.map((layout) => layout.width)),
    [widgetLayouts]
  );

  const topWidgetIdentifier = orderedWidgetIdentifiers[0];
  const bottomWidgetIdentifier =
    orderedWidgetIdentifiers[orderedWidgetIdentifiers.length - 1];

  const shouldHideWidgetList =
    config.mocked || homeWidgets.length === 0 || !isHydrated;

  if (shouldHideWidgetList) {
    return null;
  }

  return (
    <Box
      flexShrink={0}
      w={`${containerWidth}px`}
      minW={0}
      minH={0}
      h="100%"
      maxH="100%"
      overflow="hidden"
      position="relative"
    >
      <Box
        position="absolute"
        inset={0}
        overflowY="auto"
        overflowX="hidden"
        className="no-scrollbar"
      >
        <VStack align="start" spacing={4}>
          {widgetLayouts.map(({ widget, bounds, width }) => (
            <HomeWidget
              key={widget.resetKey}
              widget={widget}
              width={width}
              widthBounds={bounds}
              onWidthChange={(nextWidth) =>
                handleWidgetWidthChange(widget.identifier, nextWidth)
              }
              onWidthCommit={(nextWidth) =>
                handleWidgetWidthCommit(widget.identifier, nextWidth)
              }
              isCollapsed={!!widgetCollapsedMap[widget.identifier]}
              onCollapsedChange={(collapsed) =>
                handleWidgetCollapsedChange(widget.identifier, collapsed)
              }
              canMoveUp={widget.identifier !== topWidgetIdentifier}
              onMoveUp={() => handleMoveWidget(widget.identifier, -1)}
              canMoveDown={widget.identifier !== bottomWidgetIdentifier}
              onMoveDown={() => handleMoveWidget(widget.identifier, 1)}
            />
          ))}
        </VStack>
      </Box>
    </Box>
  );
};

export default HomeWidgetContainer;
