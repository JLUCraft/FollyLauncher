import { Flex, HStack } from "@chakra-ui/react";
import { useEffect, useRef, useState } from "react";
import HomeWidgetContainer from "@/components/extension/home-widget-container";
import HomeButtonGroup from "@/components/home-button-group";

/**
 * Width reserved next to the launch button group when computing the max widget
 * width. Keep in sync with the HStack below: p={7} (28px per side) + spacing={6}
 * (24px) + 8px safety margin for the scrollbar/rounding.
 */
const LAYOUT_RESERVED_WIDTH = 28 * 2 + 24 + 8;

const LaunchPage = () => {
  const layoutRef = useRef<HTMLDivElement>(null);
  const [maxWidgetWidth, setMaxWidgetWidth] = useState<number>(550);

  useEffect(() => {
    const layoutEl = layoutRef.current;
    if (!layoutEl) return;

    const updateMaxWidgetWidth = () => {
      const minButtonGroupWidth =
        document.getElementById("main-launch-button")?.getBoundingClientRect()
          .width ?? 14.5 * 16;
      const available =
        layoutEl.clientWidth - minButtonGroupWidth - LAYOUT_RESERVED_WIDTH;
      const safeMax = Math.max(0, available);
      setMaxWidgetWidth(safeMax);
    };

    updateMaxWidgetWidth();
    const observer = new ResizeObserver(updateMaxWidgetWidth);
    observer.observe(layoutEl);
    return () => observer.disconnect();
  }, []);

  return (
    <HStack ref={layoutRef} p={7} pt={1} align="stretch" h="100%" spacing={6}>
      <HomeWidgetContainer maxWidth={maxWidgetWidth} />

      <Flex justify="flex-end" align="flex-end" minW="14.5rem" w="100%">
        <HomeButtonGroup />
      </Flex>
    </HStack>
  );
};

export default LaunchPage;
