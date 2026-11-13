import { Box, Card, Center, Divider } from "@chakra-ui/react";
import React, { useCallback, useEffect, useRef, useState } from "react";
import { BeatLoader } from "react-spinners";
import {
  AutoSizer,
  List,
  ListRowProps,
  ScrollEventData,
} from "react-virtualized";
import { OptionItem, OptionItemProps } from "@/components/common/option-item";
import { Section, SectionProps } from "@/components/common/section";
import { debounce } from "@/utils/debounce";
import cardStyles from "@/styles/card.module.css";

export { OptionItem };
export type { OptionItemProps };

export interface VirtualOptionItemGroupProps extends SectionProps {
  items: (OptionItemProps | React.ReactNode)[];
  withDivider?: boolean;
  useInfiniteScroll?: boolean;
  hasMore?: boolean;
  loadMore?: () => void;
}

export const VirtualOptionItemGroup: React.FC<VirtualOptionItemGroupProps> = ({
  items,
  withDivider = true,
  useInfiniteScroll = false,
  hasMore = false,
  loadMore = () => {},
  ...props
}) => {
  const offscreenItemRef = useRef<HTMLDivElement>(null);
  const listRef = useRef<List>(null);
  const [itemHeight, setItemHeight] = useState(0);
  const [loadingItems, setLoadingItems] = useState<typeof items | null>(null);
  const isHeightCalculated = itemHeight > 0;
  const canLoadMore = useInfiniteScroll && hasMore && isHeightCalculated;
  const loadingMore = loadingItems === items;

  useEffect(() => {
    const element = offscreenItemRef.current;
    if (!element) return;
    const observer = new ResizeObserver(() =>
      setItemHeight(element.clientHeight)
    );
    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  const debouncedLoadMore = debounce(() => {
    if (canLoadMore && hasMore && !loadingMore) {
      setLoadingItems(items);
      loadMore();
    }
  }, 250);

  const handleScroll = useCallback(
    ({ clientHeight, scrollHeight, scrollTop }: ScrollEventData) => {
      if (scrollTop + clientHeight >= scrollHeight - 300) {
        debouncedLoadMore();
      }
    },
    [debouncedLoadMore]
  );

  function isOptionItemProps(item: unknown): item is OptionItemProps {
    return (
      (item as OptionItemProps)?.title != null &&
      (item as OptionItemProps)?.children != null
    );
  }

  const rowRenderer = ({ index, style }: ListRowProps) => {
    const item = items[index];
    return (
      <div style={style} key={index}>
        {isOptionItemProps(item) ? (
          <OptionItem {...item}>{item.children}</OptionItem>
        ) : (
          item
        )}
        {index !== items.length - 1 &&
          (withDivider ? <Divider my={2} /> : <Box h={2} />)}
      </div>
    );
  };

  return (
    <Section {...props}>
      {items.length > 0 && (
        <Card className={cardStyles["card-front"]} h="100%">
          <Box
            ref={offscreenItemRef}
            position="absolute"
            visibility="hidden"
            pointerEvents="none"
          >
            {isOptionItemProps(items[0]) && (
              <OptionItem {...items[0]}>{items[0].children}</OptionItem>
            )}
            {withDivider ? <Divider my={2} /> : <Box h={2} />}
          </Box>
          {isHeightCalculated && (
            <AutoSizer>
              {({ height, width }) => (
                <List
                  ref={listRef}
                  height={height}
                  width={width}
                  rowCount={items.length}
                  rowHeight={itemHeight}
                  rowRenderer={rowRenderer}
                  onScroll={handleScroll}
                />
              )}
            </AutoSizer>
          )}
          {loadingMore && (
            <Center key="loading" mt="auto">
              <BeatLoader size={12} color="gray" />
            </Center>
          )}
        </Card>
      )}
    </Section>
  );
};
